pub struct Monitor;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor() {
        let _ = Monitor;
    }

    #[test]
    fn test_monitor_is_zero_sized() {
        assert_eq!(std::mem::size_of::<Monitor>(), 0);
    }

    #[test]
    fn test_monitor_assignment() {
        let m1 = Monitor;
        let _m2 = m1;
        // Monitor is a unit struct, assignment is a no-op
    }
}
