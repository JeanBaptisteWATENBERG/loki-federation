use crate::federated_loki::Direction;

pub fn aggregate(set_a: Vec<(i64, String)>, set_b: Vec<(i64, String)>, direction: Direction) -> Vec<(i64, String)> {
    let mut result = set_a.clone();
    for item in set_b {
        if !result.contains(&item) {
            match direction {
                Direction::Forward =>
                    match result.iter().rposition(|&(timestamp, _)| timestamp < item.0) {
                        Some(index) => {
                            result.insert(index+1, item)
                        }
                        None =>
                            match result.iter().position(|&(timestamp, _)| timestamp <= item.0) {
                                Some(index) => {
                                    result.insert(index + 1, item)
                                }
                                None => {
                                    result.insert(0, item)
                                }
                            },
                    },

                Direction::Backward =>
                    match result.iter().position(|&(timestamp, _)| timestamp <= item.0) {
                        Some(index) => {
                            result.insert(index, item)
                        }
                        None => match result.iter().rposition(|&(timestamp, _)| timestamp > item.0) {
                            Some(index) => {
                                result.insert(index + 1, item)
                            }
                            None => {
                                result.insert(0, item)
                            }
                        },
                    },
            };
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_aggregate_when_forward_and_set_a_contains_less_items_than_set_b() {
        assert_eq!(aggregate(vec![(1, "A".to_string())], vec![(1, "A".to_string()), (2, "B".to_string())], Direction::Forward), vec![(1, "A".to_string()), (2, "B".to_string())]);
    }

    #[test]
    fn it_should_aggregate_when_forward_and_set_a_contains_a_different_item_than_set_b() {
        assert_eq!(aggregate(vec![(1, "A".to_string())], vec![(1, "A the second one".to_string()), (2, "B".to_string())], Direction::Forward), vec![(1, "A".to_string()), (1, "A the second one".to_string()), (2, "B".to_string())]);
    }

    #[test]
    fn it_should_aggregate_when_backward_and_set_a_contains_a_different_item_than_set_b() {
        assert_eq!(aggregate(vec![(1, "A".to_string())], vec![(2, "B".to_string()), (1, "A the second one".to_string())], Direction::Backward), vec![(2, "B".to_string()), (1, "A the second one".to_string()), (1, "A".to_string())]);
    }

    #[test]
    fn it_should_aggregate_when_forward_and_set_a_is_empty() {
        assert_eq!(aggregate(vec![], vec![(1, "A".to_string()), (2, "B".to_string())], Direction::Forward), vec![(1, "A".to_string()), (2, "B".to_string())]);
    }

    #[test]
    fn it_should_aggregate_when_backward_and_set_a_is_empty() {
        assert_eq!(aggregate(vec![], vec![(2, "B".to_string()), (1, "A".to_string())], Direction::Backward), vec![(2, "B".to_string()), (1, "A".to_string())]);
    }

    #[test]
    fn it_should_aggregate_when_forward_and_set_a_contains_more_items_than_set_b() {
        assert_eq!(aggregate(vec![(2, "B".to_string()), (3, "C".to_string())], vec![(1, "A".to_string())], Direction::Forward), vec![(1, "A".to_string()), (2, "B".to_string()), (3, "C".to_string())]);
    }

    #[test]
    fn it_should_aggregate_when_forward_and_set_a_and_b_contains_intersected_items() {
        assert_eq!(aggregate(vec![(2, "B".to_string()), (4, "D".to_string())], vec![(1, "A".to_string()), (3, "C".to_string())], Direction::Forward), vec![(1, "A".to_string()), (2, "B".to_string()), (3, "C".to_string()), (4, "D".to_string())]);
    }

    #[test]
    fn it_should_aggregate_when_backward_and_set_a_and_b_contains_intesected_items() {
        assert_eq!(aggregate(vec![(4, "D".to_string()), (2, "B".to_string())], vec![(3, "C".to_string()), (1, "A".to_string())], Direction::Backward), vec![(4, "D".to_string()), (3, "C".to_string()), (2, "B".to_string()), (1, "A".to_string())]);
    }

    #[test]
    fn it_should_aggregate_when_backward_and_set_a_contains_less_items_than_set_b() {
        assert_eq!(aggregate(vec![(1, "A".to_string())], vec![(1, "A".to_string()), (2, "B".to_string())], Direction::Backward), vec![(2, "B".to_string()), (1, "A".to_string())]);
    }
}