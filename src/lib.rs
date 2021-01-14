use num_traits::Bounded;
use std::cmp::Ordering;
use std::collections::VecDeque;

#[cfg(debug_assertions)]
const FLAT_ARRAY_SIZE: usize = 3;

#[cfg(not(debug_assertions))]
const FLAT_ARRAY_SIZE: usize = 50;

struct Node<Item> {
    vantage_point: Item,
    radius: f32,
}

pub struct VPTree<Item, Distance>
where
    Item: Clone,
    Distance: Fn(&Item, &Item) -> f32,
{
    distance_calculator: Distance,
    nodes: Vec<Node<Item>>,
    leaves: Vec<Vec<Item>>,
}

impl<Item, Distance> VPTree<Item, Distance>
where
    Item: Clone,
    Distance: Fn(&Item, &Item) -> f32,
{
    pub fn new(items: &[Item], distance_calculator: Distance) -> Self {
        let mut items_with_distances: Vec<(&Item, f32)> =
            items.iter().map(|i| (i, f32::max_value())).collect();
        let mut length = 0;
        let mut level = 1;
        while length + level * FLAT_ARRAY_SIZE < items.len() {
            length += level;
            level *= 2;
        }
        let mut nodes = Vec::with_capacity(length);

        let mut queue = VecDeque::new();
        queue.push_back(items_with_distances.as_mut_slice());
        while let Some(items) = queue.pop_front() {
            let (vantage_point, items) = items.split_last_mut().unwrap();
            let vantage_point = vantage_point.0.clone();

            for i in items.iter_mut() {
                i.1 = distance_calculator(&vantage_point, &i.0)
            }

            //items.select_nth_unstable_by_key(items.len()/2, |a| distance_calculator(&vantage_point, a));
            items.select_nth_unstable_by(items.len() / 2, |a, b| {
                if a.1 < b.1 {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            });
            let radius = items[items.len() / 2].1;
            let (near_items, far_items) = items.split_at_mut(items.len() / 2);
            queue.push_back(near_items);
            queue.push_back(far_items);
            nodes.push(Node {
                vantage_point: vantage_point.clone(),
                radius,
            });
            if !(queue.len() < level) {
                break;
            }
        }
        let leaves = queue
            .into_iter()
            .map(|items| items.into_iter().map(|(item, _)| item.clone()).collect())
            .collect();
        Self {
            distance_calculator,
            nodes,
            leaves,
        }
    }
    //fn consider_node_k_nearest_neighbors(&self)

    pub fn find_nearest(&self, needle: &Item, max_neighbor_count: usize) -> Vec<(f32, Item)> {
        let mut nearest_neighbors = Vec::with_capacity(max_neighbor_count);
        let mut index = 0;
        let mut node = self.nodes.get(index).unwrap();
        let mut furthest_neighbors_distance = f32::max_value();
        let mut distance;
        let mut unexplored = Vec::new();
        'outer: loop {
            distance = (self.distance_calculator)(needle, &node.vantage_point);
            if nearest_neighbors.len() < nearest_neighbors.capacity() {
                nearest_neighbors.push((distance, index));
                if nearest_neighbors.len() == nearest_neighbors.capacity() {
                    nearest_neighbors.sort_by(|a, b| {
                        if a.0 < b.0 {
                            Ordering::Less
                        } else {
                            Ordering::Greater
                        }
                    });
                    furthest_neighbors_distance = nearest_neighbors.last().unwrap().0;
                }
            } else if distance < furthest_neighbors_distance {
                nearest_neighbors.pop();
                nearest_neighbors.insert(
                    // Keep the vec sorted by inserting at index specified by binary search
                    nearest_neighbors
                        .binary_search_by(|(neighbor_distance, _)| {
                            if neighbor_distance < &distance {
                                Ordering::Less
                            } else {
                                Ordering::Greater
                            }
                        })
                        .unwrap_or_else(|x| x),
                    (distance, index),
                );
                // Update the max observed distance, unwrap is safe because this function
                // inserts a point and the `max_item_count` is more then 0.
                furthest_neighbors_distance = nearest_neighbors.last().unwrap().0;
            }
            index = if distance < node.radius {
                /* Needle is within node's radius, therefore its nearest neigbors
                are likely to be within it too. The left tree, at index*2, contains
                all child nodes within child's radius, so search that tree and add
                the right tree, at index*2+1 to the stack of unexplored nodes along
                with the distance between needle and current node's boundary. */
                index *= 2;
                unexplored.push((index + 2, node.radius - distance));
                index + 1
            } else {
                index *= 2;
                unexplored.push((index + 1, distance - node.radius));
                index + 2
            };

            if let Some(new_node) = self.nodes.get(index) {
                node = new_node;
                continue;
            } else {
                let items = self.leaves.get(index - self.nodes.len()).unwrap();
                for (inner_index, item) in items.iter().enumerate() {
                    distance = (self.distance_calculator)(needle, item);
                    if nearest_neighbors.len() < nearest_neighbors.capacity() {
                        nearest_neighbors.push((
                            distance,
                            (index - self.nodes.len()) * FLAT_ARRAY_SIZE
                                + inner_index
                                + self.nodes.len(),
                        ));
                        if nearest_neighbors.len() == nearest_neighbors.capacity() {
                            nearest_neighbors.sort_by(|a, b| {
                                if a.0 < b.0 {
                                    Ordering::Less
                                } else {
                                    Ordering::Greater
                                }
                            });
                            furthest_neighbors_distance = nearest_neighbors.last().unwrap().0;
                        }
                    } else if distance < furthest_neighbors_distance {
                        nearest_neighbors.pop();
                        nearest_neighbors.insert(
                            // Keep the vec sorted by inserting at index specified by binary search
                            nearest_neighbors
                                .binary_search_by(|(neighbor_distance, _)| {
                                    if neighbor_distance < &distance {
                                        Ordering::Less
                                    } else {
                                        Ordering::Greater
                                    }
                                })
                                .unwrap_or_else(|x| x),
                            (
                                distance,
                                (index - self.nodes.len()) * FLAT_ARRAY_SIZE
                                    + inner_index
                                    + self.nodes.len(),
                            ),
                        );
                        // Update the max observed distance, unwrap is safe because this function
                        // inserts a point and the `max_item_count` is more then 0.
                        furthest_neighbors_distance = nearest_neighbors.last().unwrap().0;
                    }
                }
            }
            while let Some((potential_index, distance_to_boundary)) = unexplored.pop() {
                if let Some(potential_node) = self.nodes.get(potential_index) {
                    /* At this point it is guaranteed that the other child of potential_node's
                    parent has been explored. Therefore, all the potential nodes on the other
                    side of the parent's boundary (defined by its radius) have been considered.
                    potential_node can possibly have viable neighbor candidates only if the
                    current furthest_neighbors_distance is so large, that it crosses over the boundary,
                    meaning that there may be a node within potential_node's domain that is closer
                    to needle than furthest_neighbors_distance. */
                    if furthest_neighbors_distance >= distance_to_boundary {
                        index = potential_index;
                        node = potential_node;
                        continue 'outer;
                    }
                } else if furthest_neighbors_distance >= distance_to_boundary {
                    let items = self.leaves.get(potential_index - self.nodes.len()).unwrap();
                    for (inner_index, item) in items.iter().enumerate() {
                        distance = (self.distance_calculator)(needle, item);
                        if nearest_neighbors.len() < nearest_neighbors.capacity() {
                            nearest_neighbors.push((
                                distance,
                                (potential_index - self.nodes.len()) * FLAT_ARRAY_SIZE
                                    + inner_index
                                    + self.nodes.len(),
                            ));
                            if nearest_neighbors.len() == nearest_neighbors.capacity() {
                                nearest_neighbors.sort_by(|a, b| {
                                    if a.0 < b.0 {
                                        Ordering::Less
                                    } else {
                                        Ordering::Greater
                                    }
                                });
                                furthest_neighbors_distance = nearest_neighbors.last().unwrap().0;
                            }
                        } else if distance < furthest_neighbors_distance {
                            nearest_neighbors.pop();
                            nearest_neighbors.insert(
                                // Keep the vec sorted by inserting at index specified by binary search
                                nearest_neighbors
                                    .binary_search_by(|(neighbor_distance, _)| {
                                        if neighbor_distance < &distance {
                                            Ordering::Less
                                        } else {
                                            Ordering::Greater
                                        }
                                    })
                                    .unwrap_or_else(|x| x),
                                (
                                    distance,
                                    (potential_index - self.nodes.len()) * FLAT_ARRAY_SIZE
                                        + inner_index
                                        + self.nodes.len(),
                                ),
                            );
                            // Update the max observed distance, unwrap is safe because this function
                            // inserts a point and the `max_item_count` is more then 0.
                            furthest_neighbors_distance = nearest_neighbors.last().unwrap().0;
                        }
                    }
                }
            }
            break;
        }
        nearest_neighbors
            .into_iter()
            .map(|(distance, mut index)| {
                (
                    distance,
                    if index < self.nodes.len() {
                        self.nodes[index].vantage_point.clone()
                    } else {
                        index -= self.nodes.len();
                        self.leaves[index / FLAT_ARRAY_SIZE][index % FLAT_ARRAY_SIZE].clone()
                    },
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_knn() {
        let points = vec![
            (2.0, 3.0),
            (0.0, 1.0),
            (4.0, 5.0),
            (45.0, 43.0),
            (21.0, 20.0),
            (39.0, 44.0),
            (96.0, 46.0),
            (95.0, 32.0),
            (14.0, 63.0),
            (19.0, 81.0),
            (66.0, 36.0),
            (26.0, 64.0),
            (10.0, 21.0),
            (92.0, 84.0),
            (31.0, 55.0),
            (59.0, 4.0),
            (43.0, 11.0),
            (87.0, 56.0),
            (76.0, 52.0),
            (10.0, 55.0),
            (64.0, 97.0),
            (6.0, 4.0),
            (10.0, 68.0),
            (9.0, 8.0),
            (60.0, 61.0),
            (22.0, 26.0),
            (79.0, 52.0),
            (29.0, 98.0),
            (88.0, 60.0),
            (29.0, 97.0),
            (42.0, 20.0),
            (5.0, 57.0),
            (81.0, 58.0),
            (22.0, 70.0),
            (44.0, 47.0),
            (16.0, 6.0),
            (2.0, 19.0),
            (26.0, 59.0),
            (45.0, 34.0),
            (10.0, 37.0),
            (8.0, 46.0),
            (38.0, 6.0),
            (98.0, 83.0),
            (18.0, 79.0),
            (3.0, 81.0),
            (77.0, 40.0),
            (82.0, 93.0),
            (1.0, 65.0),
            (51.0, 86.0),
            (34.0, 10.0),
            (91.0, 16.0),
            (28.0, 33.0),
            (5.0, 93.0),
        ];
        let tree = VPTree::new(&points, |a, b| {
            ((a.0 - b.0 as f32).powi(2) + (a.1 - b.1 as f32).powi(2)).sqrt()
        });

        let expected = vec![(13.453624, (60.0, 61.0))];
        let actual = tree.find_nearest(&(69.0, 71.0), 1);
        assert_eq!(actual, expected);

        let expected = vec![(4.2426405, (91.0, 16.0)), (13.038404, (95.0, 32.0))];
        let actual = tree.find_nearest(&(94.0, 19.0), 2);
        assert_eq!(actual, expected);

        let expected = vec![
            (4.472136, (5.0, 57.0)),
            (6.708204, (10.0, 55.0)),
            (7.2111025, (1.0, 65.0)),
            (7.28011, (14.0, 63.0)),
            (7.615773, (10.0, 68.0)),
            (15.033297, (8.0, 46.0)),
            (17.492855, (22.0, 70.0)),
            (19.104973, (26.0, 59.0)),
            (19.235384, (26.0, 64.0)),
            (20.396078, (3.0, 81.0)),
        ];
        let actual = tree.find_nearest(&(7.0, 61.0), 10);
        assert_eq!(actual, expected);

        let expected = vec![
            (3.6055512, (87.0, 56.0)),
            (5.0, (81.0, 58.0)),
            (5.3851647, (79.0, 52.0)),
            (7.2111025, (88.0, 60.0)),
            (8.246211, (76.0, 52.0)),
            (14.422205, (96.0, 46.0)),
            (15.652476, (77.0, 40.0)),
            (24.596748, (95.0, 32.0)),
            (25.0, (60.0, 61.0)),
            (25.455845, (66.0, 36.0)),
            (31.04835, (92.0, 84.0)),
            (32.202484, (98.0, 83.0)),
            (38.63936, (91.0, 16.0)),
            (39.051247, (82.0, 93.0)),
            (40.5216, (45.0, 43.0)),
            (40.60788, (44.0, 47.0)),
            (43.829212, (45.0, 34.0)),
            (45.96738, (51.0, 86.0)),
            (46.09772, (39.0, 44.0)),
            (47.423622, (64.0, 97.0)),
            (53.009434, (31.0, 55.0)),
            (54.037025, (42.0, 20.0)),
            (55.9017, (59.0, 4.0)),
            (58.21512, (26.0, 59.0)),
            (58.855755, (26.0, 64.0)),
            (59.413803, (43.0, 11.0)),
            (59.808025, (28.0, 33.0)),
            (64.03124, (22.0, 70.0)),
            (66.48308, (38.0, 6.0)),
            (66.6033, (34.0, 10.0)),
            (68.0294, (22.0, 26.0)),
            (69.81404, (29.0, 97.0)),
            (70.38466, (19.0, 81.0)),
            (70.434364, (29.0, 98.0)),
            (70.5762, (18.0, 79.0)),
            (70.5762, (14.0, 63.0)),
            (71.5891, (21.0, 20.0)),
            (74.00676, (10.0, 55.0)),
            (75.31268, (10.0, 68.0)),
            (75.9276, (10.0, 37.0)),
            (76.41989, (8.0, 46.0)),
            (79.05694, (5.0, 57.0)),
            (81.02469, (10.0, 21.0)),
            (83.23461, (16.0, 6.0)),
            (83.725746, (1.0, 65.0)),
            (85.3815, (3.0, 81.0)),
            (87.982956, (9.0, 8.0)),
            (88.10221, (5.0, 93.0)),
            (89.157166, (2.0, 19.0)),
            (92.64988, (6.0, 4.0)),
        ];
        let actual = tree.find_nearest(&(84.0, 54.0), 50);
        assert_eq!(actual, expected);
    }
}
