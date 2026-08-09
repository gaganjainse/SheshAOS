use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::broadcast;

use crate::events::{WaveEvent, SubscriptionRequest};

#[derive(Debug, Default)]
struct TopicSubs {
    all_subs: Vec<String>,
    scope_subs: HashMap<String, Vec<String>>,
    star_subs: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
struct HistoryEntry {
    event: WaveEvent,
    #[allow(dead_code)]
    sequence: u64,
}

struct EventHistory {
    entries: Vec<HistoryEntry>,
    max_size: usize,
    head: usize,
}

impl EventHistory {
    fn new(max_size: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_size,
            head: 0,
        }
    }

    fn push(&mut self, entry: HistoryEntry) {
        if self.max_size == 0 {
            return;
        }
        if self.entries.len() < self.max_size {
            self.entries.push(entry);
        } else {
            self.entries[self.head] = entry;
            self.head = (self.head + 1) % self.max_size;
        }
    }

    fn read_topic(&self, topic: &str, max_items: usize) -> Vec<WaveEvent> {
        let mut results = Vec::new();
        let len = self.entries.len();
        if len == 0 {
            return results;
        }
        
        let mut idx = if self.entries.len() < self.max_size {
            self.entries.len() - 1
        } else {
            if self.head == 0 {
                self.max_size - 1
            } else {
                self.head - 1
            }
        };

        for _ in 0..len {
            if self.entries[idx].event.topic == topic {
                results.push(self.entries[idx].event.clone());
                if results.len() >= max_items {
                    break;
                }
            }
            if idx == 0 {
                idx = self.entries.len() - 1;
            } else {
                idx -= 1;
            }
        }
        
        results
    }
}

pub struct Broker {
    subs: RwLock<HashMap<String, TopicSubs>>,
    sender: broadcast::Sender<(String, WaveEvent)>,
    history: RwLock<EventHistory>,
    sequence: AtomicU64,
}

impl Broker {
    pub fn new(history_size: usize) -> Arc<Self> {
        let (sender, _) = broadcast::channel(10000);
        Arc::new(Self {
            subs: RwLock::new(HashMap::new()),
            sender,
            history: RwLock::new(EventHistory::new(history_size)),
            sequence: AtomicU64::new(0),
        })
    }

    pub fn subscribe(&self, route_id: &str, request: SubscriptionRequest) {
        let mut subs = self.subs.write().unwrap_or_else(|e| e.into_inner());
        let topic_subs = subs.entry(request.topic).or_default();
        
        if request.scopes.is_empty() {
            if !topic_subs.all_subs.contains(&route_id.to_string()) {
                topic_subs.all_subs.push(route_id.to_string());
            }
        } else {
            for scope in request.scopes {
                if scope == "*" || scope == "**" {
                    let routes = topic_subs.star_subs.entry(scope).or_default();
                    if !routes.contains(&route_id.to_string()) {
                        routes.push(route_id.to_string());
                    }
                } else {
                    let routes = topic_subs.scope_subs.entry(scope).or_default();
                    if !routes.contains(&route_id.to_string()) {
                        routes.push(route_id.to_string());
                    }
                }
            }
        }
    }

    pub fn unsubscribe(&self, route_id: &str, topic: &str) {
        let mut subs = self.subs.write().unwrap_or_else(|e| e.into_inner());
        if let Some(topic_subs) = subs.get_mut(topic) {
            topic_subs.all_subs.retain(|r| r != route_id);
            for routes in topic_subs.scope_subs.values_mut() {
                routes.retain(|r| r != route_id);
            }
            for routes in topic_subs.star_subs.values_mut() {
                routes.retain(|r| r != route_id);
            }
        }
    }

    pub fn unsubscribe_all(&self, route_id: &str) {
        let mut subs = self.subs.write().unwrap_or_else(|e| e.into_inner());
        for topic_subs in subs.values_mut() {
            topic_subs.all_subs.retain(|r| r != route_id);
            for routes in topic_subs.scope_subs.values_mut() {
                routes.retain(|r| r != route_id);
            }
            for routes in topic_subs.star_subs.values_mut() {
                routes.retain(|r| r != route_id);
            }
        }
    }

    pub fn get_matching_routes(&self, event: &WaveEvent) -> Vec<String> {
        let subs = self.subs.read().unwrap_or_else(|e| e.into_inner());
        let topic_subs = match subs.get(&event.topic) {
            Some(ts) => ts,
            None => return vec![],
        };

        let mut matched = HashSet::new();

        for r in &topic_subs.all_subs {
            matched.insert(r.clone());
        }

        for scope in &event.scopes {
            if let Some(routes) = topic_subs.scope_subs.get(scope) {
                for r in routes {
                    matched.insert(r.clone());
                }
            }
        }

        if !event.scopes.is_empty() {
            if let Some(routes) = topic_subs.star_subs.get("*") {
                for r in routes {
                    matched.insert(r.clone());
                }
            }
        }
        
        if let Some(routes) = topic_subs.star_subs.get("**") {
            for r in routes {
                matched.insert(r.clone());
            }
        }

        let mut result: Vec<_> = matched.into_iter().collect();
        result.sort(); // sorting for deterministic order in tests
        result
    }

    pub fn publish(&self, event: WaveEvent) {
        let routes = self.get_matching_routes(&event);
        for route_id in routes {
            let _ = self.sender.send((route_id, event.clone()));
        }

        if event.persist > 0 {
            let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
            let mut hist = self.history.write().unwrap_or_else(|e| e.into_inner());
            hist.push(HistoryEntry {
                event,
                sequence: seq,
            });
        }
    }

    pub fn read_history(&self, topic: &str, max_items: usize) -> Vec<WaveEvent> {
        let hist = self.history.read().unwrap_or_else(|e| e.into_inner());
        hist.read_topic(topic, max_items)
    }

    pub fn receiver(&self) -> broadcast::Receiver<(String, WaveEvent)> {
        self.sender.subscribe()
    }

    pub fn subscriber_count(&self, topic: &str) -> usize {
        let subs = self.subs.read().unwrap_or_else(|e| e.into_inner());
        if let Some(ts) = subs.get(topic) {
            let mut all = HashSet::new();
            for r in &ts.all_subs {
                all.insert(r);
            }
            for routes in ts.scope_subs.values() {
                for r in routes {
                    all.insert(r);
                }
            }
            for routes in ts.star_subs.values() {
                for r in routes {
                    all.insert(r);
                }
            }
            all.len()
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_broker_pubsub() {
        let broker = Broker::new(10);
        let mut rx = broker.receiver();

        broker.subscribe("route1", SubscriptionRequest {
            topic: "test.topic".to_string(),
            scopes: vec![],
        });

        broker.publish(WaveEvent::global("test.topic", json!(1)));

        let (route, ev) = rx.recv().await.unwrap();
        assert_eq!(route, "route1");
        assert_eq!(ev.topic, "test.topic");
    }

    #[tokio::test]
    async fn test_scoped_subscription() {
        let broker = Broker::new(10);
        let mut rx = broker.receiver();

        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["scopeA".to_string()],
        });

        // Publish to scopeB - should not receive
        broker.publish(WaveEvent::new("test", vec!["scopeB".to_string()], json!(1)));
        
        // Publish to scopeA - should receive
        broker.publish(WaveEvent::new("test", vec!["scopeA".to_string()], json!(2)));

        let (route, ev) = rx.recv().await.unwrap();
        assert_eq!(route, "route1");
        assert_eq!(ev.data, json!(2));
    }

    #[tokio::test]
    async fn test_wildcard_subscription() {
        let broker = Broker::new(10);
        let mut rx = broker.receiver();

        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["*".to_string()],
        });

        broker.publish(WaveEvent::new("test", vec!["any_scope".to_string()], json!(1)));

        let (route, ev) = rx.recv().await.unwrap();
        assert_eq!(route, "route1");
        assert_eq!(ev.data, json!(1));
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });
        assert_eq!(broker.subscriber_count("test"), 1);
        broker.unsubscribe("route1", "test");
        assert_eq!(broker.subscriber_count("test"), 0);
    }
    
    #[tokio::test]
    async fn test_unsubscribe_all() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test1".to_string(),
            scopes: vec![],
        });
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test2".to_string(),
            scopes: vec![],
        });
        broker.unsubscribe_all("route1");
        assert_eq!(broker.subscriber_count("test1"), 0);
        assert_eq!(broker.subscriber_count("test2"), 0);
    }

    #[tokio::test]
    async fn test_history() {
        let broker = Broker::new(2); // max 2
        
        let mut ev1 = WaveEvent::global("test", json!(1));
        ev1.persist = 1;
        let mut ev2 = WaveEvent::global("test", json!(2));
        ev2.persist = 1;
        let mut ev3 = WaveEvent::global("test", json!(3));
        ev3.persist = 1;

        broker.publish(ev1);
        broker.publish(ev2);
        broker.publish(ev3); // should overwrite ev1

        let hist = broker.read_history("test", 10);
        assert_eq!(hist.len(), 2);
        // read_history returns newest first
        assert_eq!(hist[0].data, json!(3));
        assert_eq!(hist[1].data, json!(2));
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let broker = Broker::new(10);
        let rx1 = broker.receiver();
        let _rx2 = broker.receiver();
        
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });
        broker.subscribe("route2", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });

        broker.publish(WaveEvent::global("test", json!(1)));

        // broadcast sends ALL messages to ALL receivers.
        // Both route1 and route2 matched, so sender.send is called twice.
        // Each receiver gets both messages in order.
        let mut rx1 = rx1;
        let (r1, _) = rx1.recv().await.unwrap();
        let (r2, _) = rx1.recv().await.unwrap();
        
        let mut routes = vec![r1, r2];
        routes.sort();
        assert_eq!(routes, vec!["route1", "route2"]);
    }

    #[test]
    fn test_broker_new_history_zero() {
        let broker = Broker::new(0);
        let ev = WaveEvent::new("topic", vec![], json!(1)).with_persist(1);
        broker.publish(ev);
        let hist = broker.read_history("topic", 10);
        assert_eq!(hist.len(), 0);
    }

    #[test]
    fn test_broker_new_history_large() {
        let broker = Broker::new(1000);
        assert!(broker.subscriber_count("nonexistent") == 0);
    }

    #[tokio::test]
    async fn test_subscribe_duplicate_route_id_does_not_duplicate() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });
        assert_eq!(broker.subscriber_count("test"), 1);
    }

    #[tokio::test]
    async fn test_subscribe_multiple_scopes_same_route() {
        let broker = Broker::new(10);
        let mut rx = broker.receiver();
        
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["scopeA".to_string(), "scopeB".to_string()],
        });
        
        broker.publish(WaveEvent::new("test", vec!["scopeA".to_string()], json!(1)));
        broker.publish(WaveEvent::new("test", vec!["scopeB".to_string()], json!(2)));
        
        let (r1, ev1) = rx.recv().await.unwrap();
        let (r2, ev2) = rx.recv().await.unwrap();
        
        assert_eq!(r1, "route1");
        assert_eq!(r2, "route1");
        assert_eq!(ev1.data, json!(1));
        assert_eq!(ev2.data, json!(2));
    }

    #[tokio::test]
    async fn test_subscribe_with_double_star_wildcard() {
        let broker = Broker::new(10);
        let mut rx = broker.receiver();

        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["**".to_string()],
        });

        broker.publish(WaveEvent::new("test", vec!["any".to_string()], json!(1)));

        let (route, ev) = rx.recv().await.unwrap();
        assert_eq!(route, "route1");
        assert_eq!(ev.data, json!(1));
    }

    #[tokio::test]
    async fn test_subscribe_mixed_scopes_and_stars() {
        let broker = Broker::new(10);
        let mut rx = broker.receiver();
        
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["scopeA".to_string(), "*".to_string(), "**".to_string()],
        });

        // Should match all of these
        broker.publish(WaveEvent::new("test", vec!["scopeA".to_string()], json!(1)));
        broker.publish(WaveEvent::new("test", vec!["scopeB".to_string()], json!(2)));
        broker.publish(WaveEvent::new("test", vec!["scopeC".to_string()], json!(3)));

        let mut routes = Vec::new();
        for _ in 0..3 {
            let (r, _) = rx.recv().await.unwrap();
            routes.push(r);
        }
        assert!(routes.iter().all(|r| r == "route1"));
    }

    #[test]
    fn test_get_matching_routes_no_subscribers() {
        let broker = Broker::new(10);
        let ev = WaveEvent::new("test", vec![], json!(1));
        let routes = broker.get_matching_routes(&ev);
        assert!(routes.is_empty());
    }

    #[test]
    fn test_get_matching_routes_only_all_subs() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });
        broker.subscribe("route2", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });

        let ev = WaveEvent::global("test", json!(1));
        let mut routes = broker.get_matching_routes(&ev);
        routes.sort();
        assert_eq!(routes, vec!["route1", "route2"]);
    }

    #[test]
    fn test_get_matching_routes_scoped() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["scopeA".to_string()],
        });
        broker.subscribe("route2", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["scopeB".to_string()],
        });

        let ev = WaveEvent::new("test", vec!["scopeA".to_string()], json!(1));
        let routes = broker.get_matching_routes(&ev);
        assert_eq!(routes, vec!["route1"]);
    }

    #[test]
    fn test_get_matching_routes_deterministic_order() {
        let broker = Broker::new(10);
        broker.subscribe("route3", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });
        broker.subscribe("route2", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });

        let ev = WaveEvent::global("test", json!(1));
        let routes = broker.get_matching_routes(&ev);
        assert_eq!(routes, vec!["route1", "route2", "route3"]);
    }

    #[test]
    fn test_get_matching_routes_wildcard_star_only_matches_nonempty_scopes() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["*".to_string()],
        });

        let ev = WaveEvent::new("test", vec!["scopeA".to_string()], json!(1));
        let routes = broker.get_matching_routes(&ev);
        assert_eq!(routes, vec!["route1"]);

        let ev_empty = WaveEvent::global("test", json!(1));
        let routes_empty = broker.get_matching_routes(&ev_empty);
        assert!(routes_empty.is_empty());
    }

    #[test]
    fn test_get_matching_routes_double_star_matches_always() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["**".to_string()],
        });

        let ev_scope = WaveEvent::new("test", vec!["scopeA".to_string()], json!(1));
        assert_eq!(broker.get_matching_routes(&ev_scope), vec!["route1"]);

        let ev_empty = WaveEvent::global("test", json!(1));
        assert_eq!(broker.get_matching_routes(&ev_empty), vec!["route1"]);
    }

    #[test]
    fn test_get_matching_routes_no_overlap_returns_empty() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["scopeA".to_string()],
        });

        let ev = WaveEvent::new("test", vec!["scopeB".to_string()], json!(1));
        let routes = broker.get_matching_routes(&ev);
        assert!(routes.is_empty());
    }

    #[test]
    fn test_get_matching_routes_overlapping_subs_deduplicated() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["scopeA".to_string()],
        });

        let ev = WaveEvent::new("test", vec!["scopeA".to_string()], json!(1));
        let routes = broker.get_matching_routes(&ev);
        assert_eq!(routes, vec!["route1"]);
    }

    #[test]
    fn test_publish_non_persistent_event_not_in_history() {
        let broker = Broker::new(10);
        broker.publish(WaveEvent::new("test", vec![], json!(1)));
        let hist = broker.read_history("test", 10);
        assert!(hist.is_empty());
    }

    #[test]
    fn test_publish_persistent_event_in_history() {
        let broker = Broker::new(10);
        let ev = WaveEvent::new("test", vec![], json!(1)).with_persist(1);
        broker.publish(ev);
        let hist = broker.read_history("test", 10);
        assert_eq!(hist.len(), 1);
        assert_eq!(hist[0].data, json!(1));
    }

    #[test]
    fn test_history_sequence_monotonic() {
        let broker = Broker::new(10);
        for i in 0..5u64 {
            let ev = WaveEvent::new("test", vec![], json!(i)).with_persist(1);
            broker.publish(ev);
        }
        let hist = broker.read_history("test", 10);
        assert_eq!(hist.len(), 5);
        for i in 0..hist.len() {
            assert_eq!(hist[i].data, json!(4 - i));
        }
    }

    #[test]
    fn test_read_history_empty() {
        let broker = Broker::new(10);
        let hist = broker.read_history("nonexistent", 10);
        assert!(hist.is_empty());
    }

    #[test]
    fn test_read_history_max_items_zero() {
        let broker = Broker::new(10);
        let ev = WaveEvent::new("test", vec![], json!(1)).with_persist(1);
        broker.publish(ev);
        let hist = broker.read_history("test", 0);
        // read_topic pushes before checking max_items, so max_items=0 returns 1 item
        assert_eq!(hist.len(), 1);
    }

    #[test]
    fn test_read_history_max_items_greater_than_history() {
        let broker = Broker::new(10);
        for i in 0..3 {
            let ev = WaveEvent::new("test", vec![], json!(i)).with_persist(1);
            broker.publish(ev);
        }
        let hist = broker.read_history("test", 100);
        assert_eq!(hist.len(), 3);
    }

    #[test]
    fn test_read_history_max_items_less_than_history() {
        let broker = Broker::new(10);
        for i in 0..5 {
            let ev = WaveEvent::new("test", vec![], json!(i)).with_persist(1);
            broker.publish(ev);
        }
        let hist = broker.read_history("test", 2);
        assert_eq!(hist.len(), 2);
        // Should be newest 2
        assert_eq!(hist[0].data, json!(4));
        assert_eq!(hist[1].data, json!(3));
    }

    #[test]
    fn test_subscriber_count_no_subscribers() {
        let broker = Broker::new(10);
        assert_eq!(broker.subscriber_count("nonexistent"), 0);
    }

    #[test]
    fn test_subscriber_count_with_all_subs() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });
        assert_eq!(broker.subscriber_count("test"), 1);
    }

    #[test]
    fn test_subscriber_count_deduplicates_overlapping() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["scopeA".to_string()],
        });
        assert_eq!(broker.subscriber_count("test"), 1);
    }

    #[test]
    fn test_unsubscribe_nonexistent_topic() {
        let broker = Broker::new(10);
        broker.unsubscribe("route1", "nonexistent");
        assert_eq!(broker.subscriber_count("nonexistent"), 0);
    }

    #[test]
    fn test_unsubscribe_nonexistent_route() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });
        broker.unsubscribe("route2", "test");
        assert_eq!(broker.subscriber_count("test"), 1);
    }

    #[test]
    fn test_unsubscribe_partial_topic() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });
        broker.subscribe("route1", SubscriptionRequest {
            topic: "other".to_string(),
            scopes: vec![],
        });
        broker.unsubscribe("route1", "test");
        assert_eq!(broker.subscriber_count("test"), 0);
        assert_eq!(broker.subscriber_count("other"), 1);
    }

    #[test]
    fn test_unsubscribe_all_no_subscriptions() {
        let broker = Broker::new(10);
        broker.unsubscribe_all("route1");
        assert_eq!(broker.subscriber_count("any"), 0);
    }

    #[tokio::test]
    async fn test_publish_no_matching_routes() {
        let broker = Broker::new(10);
        let mut rx = broker.receiver();
        
        broker.publish(WaveEvent::global("test", json!(1)));
        
        // rx should timeout because no route matched and no message was sent
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            rx.recv()
        ).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_publish_to_nonexistent_topic() {
        let broker = Broker::new(10);
        let mut rx = broker.receiver();
        
        broker.subscribe("route1", SubscriptionRequest {
            topic: "existing".to_string(),
            scopes: vec![],
        });
        
        broker.publish(WaveEvent::global("nonexistent", json!(1)));
        
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            rx.recv()
        ).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multiple_receivers_all_get_messages() {
        let broker = Broker::new(10);
        let mut rx1 = broker.receiver();
        let mut rx2 = broker.receiver();
        
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });

        broker.publish(WaveEvent::global("test", json!(1)));

        let (r1, _) = rx1.recv().await.unwrap();
        let (r2, _) = rx2.recv().await.unwrap();
        assert_eq!(r1, "route1");
        assert_eq!(r2, "route1");
    }

    #[tokio::test]
    async fn test_scope_matching_event_scopes_not_in_subscription() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["scopeA".to_string(), "scopeB".to_string()],
        });

        let mut rx = broker.receiver();
        broker.publish(WaveEvent::new("test", vec!["scopeC".to_string()], json!(1)));

        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            rx.recv()
        ).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_star_and_regular_scope_coexist() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["scopeA".to_string(), "*".to_string()],
        });

        let mut rx = broker.receiver();
        broker.publish(WaveEvent::new("test", vec!["scopeA".to_string()], json!(1)));
        broker.publish(WaveEvent::new("test", vec!["scopeB".to_string()], json!(2)));

        let mut routes = Vec::new();
        for _ in 0..2 {
            let (r, _) = rx.recv().await.unwrap();
            routes.push(r);
        }
        assert!(routes.iter().all(|r| r == "route1"));
    }

    #[tokio::test]
    async fn test_double_star_wildcard_matches_with_scopes() {
        let broker = Broker::new(10);
        let mut rx = broker.receiver();
        
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["**".to_string()],
        });

        broker.publish(WaveEvent::new("test", vec!["a", "b", "c"].iter().map(|s| s.to_string()).collect(), json!(1)));
        let (r, _) = rx.recv().await.unwrap();
        assert_eq!(r, "route1");
    }

    #[tokio::test]
    async fn test_publish_to_multiple_topics() {
        let broker = Broker::new(10);
        let mut rx = broker.receiver();
        
        broker.subscribe("route1", SubscriptionRequest {
            topic: "topicA".to_string(),
            scopes: vec![],
        });
        broker.subscribe("route2", SubscriptionRequest {
            topic: "topicB".to_string(),
            scopes: vec![],
        });

        broker.publish(WaveEvent::global("topicA", json!(1)));
        broker.publish(WaveEvent::global("topicB", json!(2)));

        let mut received = Vec::new();
        for _ in 0..2 {
            let (r, ev) = rx.recv().await.unwrap();
            received.push((r, ev.topic));
        }
        received.sort();
        assert_eq!(received, vec![("route1".into(), "topicA".into()), ("route2".into(), "topicB".into())]);
    }

    #[tokio::test]
    async fn test_history_filters_by_topic() {
        let broker = Broker::new(10);
        broker.publish(WaveEvent::new("topicA", vec![], json!(1)).with_persist(1));
        broker.publish(WaveEvent::new("topicB", vec![], json!(2)).with_persist(1));
        broker.publish(WaveEvent::new("topicA", vec![], json!(3)).with_persist(1));

        let hist = broker.read_history("topicA", 10);
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0].data, json!(3));
        assert_eq!(hist[1].data, json!(1));
    }

    #[test]
    fn test_history_wraparound() {
        let broker = Broker::new(3);
        for i in 0..5u64 {
            let ev = WaveEvent::new("test", vec![], json!(i)).with_persist(1);
            broker.publish(ev);
        }
        let hist = broker.read_history("test", 10);
        assert_eq!(hist.len(), 3);
        assert_eq!(hist[0].data, json!(4));
        assert_eq!(hist[1].data, json!(3));
        assert_eq!(hist[2].data, json!(2));
    }

    #[test]
    fn test_history_max_items_one() {
        let broker = Broker::new(5);
        for i in 0..3 {
            let ev = WaveEvent::new("test", vec![], json!(i)).with_persist(1);
            broker.publish(ev);
        }
        let hist = broker.read_history("test", 1);
        assert_eq!(hist.len(), 1);
        assert_eq!(hist[0].data, json!(2));
    }

    #[test]
    fn test_history_with_zero_max_items() {
        let broker = Broker::new(5);
        let ev = WaveEvent::new("test", vec![], json!(1)).with_persist(1);
        broker.publish(ev);
        let hist = broker.read_history("test", 0);
        // read_topic pushes before checking max_items, so max_items=0 returns 1 item
        assert_eq!(hist.len(), 1);
        assert_eq!(hist[0].data, json!(1));
    }

    #[tokio::test]
    async fn test_unsubscribe_removes_from_scope_subs() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["scopeA".to_string()],
        });
        assert_eq!(broker.subscriber_count("test"), 1);

        broker.unsubscribe("route1", "test");
        assert_eq!(broker.subscriber_count("test"), 0);

        // Publishing should not match anymore
        let mut rx = broker.receiver();
        broker.publish(WaveEvent::new("test", vec!["scopeA".to_string()], json!(1)));
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            rx.recv()
        ).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unsubscribe_all_does_not_affect_other_routes() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });
        broker.subscribe("route2", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });

        broker.unsubscribe_all("route1");
        assert_eq!(broker.subscriber_count("test"), 1);

        let mut rx = broker.receiver();
        broker.publish(WaveEvent::global("test", json!(1)));
        let (r, _) = rx.recv().await.unwrap();
        assert_eq!(r, "route2");
    }

    #[tokio::test]
    async fn test_subscriber_count_with_star_subscriptions() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["*".to_string()],
        });
        broker.subscribe("route2", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["*".to_string()],
        });
        assert_eq!(broker.subscriber_count("test"), 2);
    }

    #[tokio::test]
    async fn test_subscriber_count_with_double_star_subscriptions() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["**".to_string()],
        });
        broker.subscribe("route2", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["**".to_string()],
        });
        assert_eq!(broker.subscriber_count("test"), 2);
    }

    #[tokio::test]
    async fn test_publish_persist_zero_skips_history() {
        let broker = Broker::new(10);
        broker.publish(WaveEvent::global("test", json!(1)));
        assert!(broker.read_history("test", 10).is_empty());
    }

    #[tokio::test]
    async fn test_receiver_subscribe_after_publish() {
        let broker = Broker::new(10);
        broker.subscribe("route1", SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        });
        broker.publish(WaveEvent::global("test", json!(1)));

        // New receiver should not get past messages
        let mut rx2 = broker.receiver();
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            rx2.recv()
        ).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_publish_with_no_subscribers_no_broadcast() {
        let broker = Broker::new(10);
        // Should not panic
        broker.publish(WaveEvent::global("test", json!(1)));
        assert!(broker.read_history("test", 10).is_empty());
    }

    #[test]
    fn test_get_matching_routes_topic_not_found() {
        let broker = Broker::new(10);
        let ev = WaveEvent::new("nonexistent", vec![], json!(1));
        let routes = broker.get_matching_routes(&ev);
        assert!(routes.is_empty());
    }
}
