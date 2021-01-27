use num_traits::Bounded;
use std::cmp::{min, Ordering};
use std::collections::VecDeque;
use std::ops::Sub;

#[cfg(debug_assertions)]
const FLAT_ARRAY_SIZE: usize = 3;

#[cfg(not(debug_assertions))]
const FLAT_ARRAY_SIZE: usize = 50;

struct Node<Item, Distance> {
    vantage_point: Item,
    radius: Distance,
}

pub struct VPTree<Item, Distance, DistanceCalculator>
where
    Item: Clone,
    Distance: PartialOrd + Bounded + Sub<Output = Distance>,
    DistanceCalculator: Fn(&Item, &Item) -> Distance,
{
    distance_calculator: DistanceCalculator,
    nodes: Vec<Node<Item, Distance>>,
    leaves: Vec<Item>,
    leaf_size: usize,
    decrementation_point: usize,
    depth: usize,
}

impl<Item, Distance, DistanceCalculator> VPTree<Item, Distance, DistanceCalculator>
where
    Item: Clone,
    Distance: Copy + PartialOrd + Bounded + Sub<Output = Distance>,
    DistanceCalculator: Fn(&Item, &Item) -> Distance,
{
    pub fn new(items: &[Item], distance_calculator: DistanceCalculator) -> Self {
        let mut items: Vec<(&Item, Distance)> =
            items.iter().map(|i| (i, Distance::max_value())).collect();
        /* Depth is the number of layers in the tree, excluding the leaf layer,
        such that every leaf contains around FLAT_ARRAY_SIZE */
        let depth = ((items.len() + 1) as f32 / (FLAT_ARRAY_SIZE + 1) as f32)
            .log2()
            .ceil() as usize;
        let leaves_len = 2usize.pow(depth as u32);
        let nodes_len = leaves_len - 1;
        // (items.len() - nodes_len) / leaves_len rounded up
        let leaf_size = items.len() / leaves_len;
        /* Root node has 2 children, those 2 children have 4 children in total and so on,
        for a total of 2^depth-1 nodes in a tree, if all layers are full, which is guaranteed
        in this implementation. */
        let mut nodes = Vec::with_capacity(nodes_len);
        /* The leaf layer is one additional layer below all the nodes, so its size is 2^depth.
        when queue grows to this size, its guaranteed to contain only data meant for the leaves. */
        let mut queue = VecDeque::with_capacity(leaves_len);
        let mut ideal_size_low = nodes_len + leaves_len * (leaf_size - 1);
        let mut ideal_size_high = nodes_len + leaves_len * leaf_size;
        let decrementation_point = items.len() - ideal_size_low;
        queue.push_back(items.as_mut_slice());
        while nodes.len() < nodes.capacity() {
            if queue.len().is_power_of_two() {
                ideal_size_low = (ideal_size_low - 1) / 2;
                ideal_size_high = (ideal_size_high - 1) / 2;
            }
            /* queue starts with one item and gains two items every iteration, the slices it contains
            get smaller every iteration, but the the loop will stop before they are smaller than
            leaf_size - 1, thus the unwraps are safe. */
            let (vantage_point, items) = queue.pop_front().unwrap().split_last_mut().unwrap();
            let split_point = min(items.len() - ideal_size_low, ideal_size_high);

            for i in items.iter_mut() {
                i.1 = distance_calculator(&vantage_point.0, &i.0)
            }
            /* Find the median distance of an item to the vantage point. Put all items
            with distance less than that on the left of the median */
            items.select_nth_unstable_by(split_point, |a, b| {
                if a.1 < b.1 {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            });
            // The median distance of an item to the vantage point
            let radius = items[split_point].1;
            let (near_items, far_items) = items.split_at_mut(split_point);
            queue.push_back(near_items);
            queue.push_back(far_items);
            nodes.push(Node {
                vantage_point: vantage_point.0.clone(),
                radius,
            });
        }
        /* Searching a tree becomes more efficient than linear search only for large amounts of items.
        For this reason, the leaves of the tree are vecs of items. */
        let leaves = queue
            .into_iter()
            .flat_map(|items| items.into_iter().map(|(item, _)| item.clone()))
            .collect();
        Self {
            distance_calculator,
            nodes,
            leaves,
            leaf_size,
            decrementation_point,
            depth,
        }
    }

    fn rebalance(&mut self) {
        let mut items: Vec<(Item, Distance)> = self
            .nodes
            .drain(..)
            .map(|node| (node.vantage_point, Distance::max_value()))
            .chain(
                self.leaves
                    .drain(..)
                    .map(|item| (item, Distance::max_value())),
            )
            .collect();

        self.depth = ((items.len() + 1) as f32 / (FLAT_ARRAY_SIZE + 1) as f32)
            .log2()
            .ceil() as usize;
        let leaves_len = 2usize.pow(self.depth as u32);
        let nodes_len = leaves_len - 1;
        self.leaf_size = ((items.len() - nodes_len) as f32 / leaves_len as f32).ceil() as usize;
        /* reserve_exact does not actually guarantee that self.nodes will have a capacity equal to
        new_nodes_length. Hence, this variable will be used where nodes.capacity() is used in VPTree::new() */
        self.nodes.reserve_exact(nodes_len);
        let mut queue = VecDeque::with_capacity(nodes_len + 1);
        let mut ideal_size = nodes_len + (nodes_len + 1) * (self.leaf_size - 1);
        let mut level = 0;
        queue.push_back(items.as_mut_slice());
        while self.nodes.len() < nodes_len {
            if queue.len().is_power_of_two() {
                ideal_size = (ideal_size - 1) / 2;
                level += 1;
            }
            let (vantage_point, items) = queue.pop_front().unwrap().split_last_mut().unwrap();

            for i in items.iter_mut() {
                i.1 = (self.distance_calculator)(&vantage_point.0, &i.0)
            }

            /* The amount of items a child can be given such that its every leaf would have
            size FLAT_ARRAY_SIZE */
            let split_point = min(
                items.len() - ideal_size,
                ideal_size + 2usize.pow((self.depth - level) as u32),
            );

            items.select_nth_unstable_by(split_point, |a, b| {
                if a.1 < b.1 {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            });
            let radius = items[split_point].1;
            let (near_items, far_items) = items.split_at_mut(split_point);
            queue.push_back(near_items);
            queue.push_back(far_items);
            self.nodes.push(Node {
                vantage_point: vantage_point.0.clone(),
                radius,
            });
        }
        self.decrementation_point = queue
            .iter()
            .enumerate()
            .find_map(|(index, leaf)| {
                if leaf.len() < self.leaf_size {
                    Some(index)
                } else {
                    None
                }
            })
            .unwrap_or(leaves_len);
        self.leaves.append(
            &mut queue
                .into_iter()
                .flat_map(|items| items.into_iter().map(|(item, _)| item.clone()))
                .collect(),
        );
    }

    pub fn insert(&mut self, item: Item) {
        self.leaves.push(item);
        self.rebalance();
    }

    pub fn extend<I: IntoIterator<Item = Item>>(&mut self, items: I) {
        self.leaves.extend(items.into_iter());
        self.rebalance();
    }

    pub fn len(&self) -> usize {
        self.nodes.len() + self.leaves.len()
    }

    pub fn find_nearest_neighbor(&self, needle: &Item) -> Option<(Distance, Item)> {
        let mut index = 0;
        let mut nearest_neighbor = index;
        let mut nearest_neighbors_distance = Distance::max_value();
        let mut unexplored = Vec::with_capacity(self.depth);
        while let Some(node) = match self.nodes.get(index) {
            Some(node) => Some(node),
            None => {
                index -= self.nodes.len();
                for (inner_index, item) in self.leaves[if index < self.decrementation_point {
                    index *= self.leaf_size;
                    index..index + self.leaf_size
                } else {
                    index = (index - self.decrementation_point) * (self.leaf_size - 1)
                        + self.decrementation_point * self.leaf_size;
                    index..index + self.leaf_size - 1
                }]
                .iter()
                .enumerate()
                {
                    let distance = (self.distance_calculator)(needle, item);
                    if distance < nearest_neighbors_distance {
                        /* This operation encodes the index of the leaf and the item in
                        that leaf in a single index in the most compact way */
                        nearest_neighbor = index + inner_index + self.nodes.len();
                        nearest_neighbors_distance = distance;
                    }
                }
                loop {
                    if let Some((mut potential_index, distance_to_boundary)) = unexplored.pop() {
                        /* At this point it is guaranteed that the other child of potential_index's
                        parent has been explored. Therefore, all the nodes on the other
                        side of the parent's boundary (defined by its radius) have been considered.
                        potential_index can possibly point to a viable neighbor candidate only if the
                        current nearest neighbor's distance is so large, that it crosses over the boundary,
                        meaning that there may be an item pointed to by potential_index that is closer
                        to needle than current nearest neighbor. */
                        if nearest_neighbors_distance > distance_to_boundary {
                            if let Some(potential_node) = self.nodes.get(potential_index) {
                                index = potential_index;
                                break Some(potential_node);
                            } else {
                                potential_index -= self.nodes.len();
                                for (inner_index, item) in self.leaves[if potential_index
                                    < self.decrementation_point
                                {
                                    potential_index *= self.leaf_size;
                                    potential_index..potential_index + self.leaf_size
                                } else {
                                    potential_index = (potential_index - self.decrementation_point)
                                        * (self.leaf_size - 1)
                                        + self.decrementation_point * self.leaf_size;
                                    potential_index..potential_index + self.leaf_size - 1
                                }]
                                .iter()
                                .enumerate()
                                {
                                    let distance = (self.distance_calculator)(needle, item);
                                    if distance < nearest_neighbors_distance {
                                        /* This operation encodes the index of the leaf and the item in
                                        that leaf in a single index in the most compact way */
                                        nearest_neighbor =
                                            potential_index + inner_index + self.nodes.len();
                                        nearest_neighbors_distance = distance;
                                    }
                                }
                            }
                        }
                    } else {
                        break None;
                    }
                }
            }
        } {
            let distance = (self.distance_calculator)(needle, &node.vantage_point);
            if distance < nearest_neighbors_distance {
                nearest_neighbor = index;
                nearest_neighbors_distance = distance;
            }
            index = if distance < node.radius {
                /* Needle is within node's radius, therefore its nearest neigbors
                are likely to be within it too. The left tree, at index*2+1, contains
                all child nodes within node's radius, so search that tree and add
                the right tree - at index*2+2 - to the stack of unexplored nodes along
                with the distance between needle and current node's boundary. */
                index *= 2;
                unexplored.push((index + 2, node.radius - distance));
                index + 1
            } else {
                index *= 2;
                unexplored.push((index + 1, distance - node.radius));
                index + 2
            };
        }
        if nearest_neighbors_distance < Distance::max_value() {
            Some((
                nearest_neighbors_distance,
                if nearest_neighbor < self.nodes.len() {
                    self.nodes[nearest_neighbor].vantage_point.clone()
                } else {
                    self.leaves[nearest_neighbor - self.nodes.len()].clone()
                },
            ))
        } else {
            None
        }
    }

    pub fn find_k_nearest_neighbors(&self, needle: &Item, k: usize) -> Vec<(Distance, Item)> {
        #[inline(always)]
        fn consider_item<Distance: PartialOrd>(
            index: usize,
            distance: Distance,
            nearest_neighbors: &mut Vec<(Distance, usize)>,
        ) {
            if nearest_neighbors.len() < nearest_neighbors.capacity() {
                nearest_neighbors.push((distance, index));
                if nearest_neighbors.len() == nearest_neighbors.capacity() {
                    /* Now that nearest_neigbors has reached its capacity,
                    the distance of its members from the needle becomes important */
                    nearest_neighbors.sort_by(|a, b| {
                        if a.0 < b.0 {
                            Ordering::Less
                        } else {
                            Ordering::Greater
                        }
                    });
                }
            } else if distance < nearest_neighbors.last().unwrap().0 {
                /* Since nearest_neigbors is guaranteed to be sorted by distance
                of its members to the needle at this point, its last member
                has the greatest (least desirable) distance to the needle.*/
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
            }
        }
        let mut nearest_neighbors = Vec::with_capacity(k);
        let mut index = 0;
        let mut unexplored = Vec::with_capacity(self.depth);
        while let Some(node) = match self.nodes.get(index) {
            Some(node) => Some(node),
            None => {
                index -= self.nodes.len();
                for (inner_index, item) in self.leaves[if index < self.decrementation_point {
                    index *= self.leaf_size;
                    index..index + self.leaf_size
                } else {
                    index = (index - self.decrementation_point) * (self.leaf_size - 1)
                        + self.decrementation_point * self.leaf_size;
                    index..index + self.leaf_size - 1
                }]
                .iter()
                .enumerate()
                {
                    consider_item(
                        index + inner_index + self.nodes.len(),
                        (self.distance_calculator)(needle, item),
                        &mut nearest_neighbors,
                    );
                }
                loop {
                    if let Some((mut potential_index, distance_to_boundary)) = unexplored.pop() {
                        /* At this point it is guaranteed that the other child of potential_index's
                        parent has been explored. Therefore, all the nodes on the other
                        side of the parent's boundary (defined by its radius) have been considered.
                        potential_index can possibly point to viable neighbor candidates only if the
                        current farthest neighbor in nearest_neighbors has a distance so large,
                        that it crosses over the boundary, meaning that there may be an item pointed
                        to by potential_index that is closer to needle than current farthest neighbor. */
                        if nearest_neighbors.last().unwrap().0 > distance_to_boundary
                            || nearest_neighbors.len() < nearest_neighbors.capacity()
                        {
                            if let Some(potential_node) = self.nodes.get(potential_index) {
                                index = potential_index;
                                break Some(potential_node);
                            } else {
                                potential_index -= self.nodes.len();
                                for (inner_index, item) in self.leaves[if potential_index
                                    < self.decrementation_point
                                {
                                    potential_index *= self.leaf_size;
                                    potential_index..potential_index + self.leaf_size
                                } else {
                                    potential_index = (potential_index - self.decrementation_point)
                                        * (self.leaf_size - 1)
                                        + self.decrementation_point * self.leaf_size;
                                    potential_index..potential_index + self.leaf_size - 1
                                }]
                                .iter()
                                .enumerate()
                                {
                                    consider_item(
                                        potential_index + inner_index + self.nodes.len(),
                                        (self.distance_calculator)(needle, item),
                                        &mut nearest_neighbors,
                                    );
                                }
                            }
                        }
                    } else {
                        break None;
                    }
                }
            }
        } {
            let distance = (self.distance_calculator)(needle, &node.vantage_point);
            consider_item(index, distance, &mut nearest_neighbors);
            index = if distance < node.radius {
                /* Needle is within node's radius, therefore its nearest neigbors
                are likely to be within it too. The left tree, at index*2+1, contains
                all child nodes within node's radius, so search that tree and add
                the right tree - at index*2+2 - to the stack of unexplored nodes along
                with the distance between needle and current node's boundary. */
                index *= 2;
                unexplored.push((index + 2, node.radius - distance));
                index + 1
            } else {
                index *= 2;
                unexplored.push((index + 1, distance - node.radius));
                index + 2
            };
        }
        nearest_neighbors
            .into_iter()
            .map(|(distance, index)| {
                (
                    distance,
                    if index < self.nodes.len() {
                        self.nodes[index].vantage_point.clone()
                    } else {
                        self.leaves[index - self.nodes.len()].clone()
                    },
                )
            })
            .collect()
    }

    pub fn find_neighbors_within_radius(
        &self,
        needle: &Item,
        threshold: Distance,
    ) -> Vec<(Distance, Item)> {
        let mut nearest_neighbors = Vec::new();
        let mut index = 0;
        let mut unexplored = Vec::with_capacity(self.depth);
        while let Some(node) = match self.nodes.get(index) {
            Some(node) => Some(node),
            None => {
                index -= self.nodes.len();
                for (inner_index, item) in self.leaves[if index < self.decrementation_point {
                    index *= self.leaf_size;
                    index..index + self.leaf_size
                } else {
                    index = (index - self.decrementation_point) * (self.leaf_size - 1)
                        + self.decrementation_point * self.leaf_size;
                    index..index + self.leaf_size - 1
                }]
                .iter()
                .enumerate()
                {
                    let distance = (self.distance_calculator)(needle, item);
                    if distance <= threshold {
                        nearest_neighbors.push((distance, index + inner_index + self.nodes.len()));
                    }
                }
                loop {
                    if let Some((mut potential_index, distance_to_boundary)) = unexplored.pop() {
                        /* We're only interested in nodes than lie within threshold distance to the needle.
                        Needle is guaranteed to be at the other side of potential_index's parent's boundary.
                        Therefore, potential_index can possibly point to viable neighbor candidates only if the
                        threshold is so large, that it crosses over the boundary. */
                        if threshold >= distance_to_boundary {
                            if let Some(potential_node) = self.nodes.get(potential_index) {
                                index = potential_index;
                                break Some(potential_node);
                            } else {
                                potential_index -= self.nodes.len();
                                for (inner_index, item) in self.leaves[if potential_index
                                    < self.decrementation_point
                                {
                                    potential_index *= self.leaf_size;
                                    potential_index..potential_index + self.leaf_size
                                } else {
                                    potential_index = (potential_index - self.decrementation_point)
                                        * (self.leaf_size - 1)
                                        + self.decrementation_point * self.leaf_size;
                                    potential_index..potential_index + self.leaf_size - 1
                                }]
                                .iter()
                                .enumerate()
                                {
                                    let distance = (self.distance_calculator)(needle, item);
                                    if distance <= threshold {
                                        nearest_neighbors.push((
                                            distance,
                                            potential_index + inner_index + self.nodes.len(),
                                        ));
                                    }
                                }
                            }
                        }
                    } else {
                        break None;
                    }
                }
            }
        } {
            let distance = (self.distance_calculator)(needle, &node.vantage_point);
            if distance <= threshold {
                nearest_neighbors.push((distance, index));
            }
            index = if distance < node.radius {
                /* Needle is within node's radius, therefore its nearest neigbors
                are likely to be within it too. The left tree, at index*2+1, contains
                all child nodes within node's radius, so search that tree and add
                the right tree - at index*2+2 - to the stack of unexplored nodes along
                with the distance between needle and current node's boundary. */
                index *= 2;
                unexplored.push((index + 2, node.radius - distance));
                index + 1
            } else {
                index *= 2;
                unexplored.push((index + 1, distance - node.radius));
                index + 2
            };
        }
        nearest_neighbors.sort_by(|a, b| {
            if a.0 < b.0 {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        });
        nearest_neighbors
            .into_iter()
            .map(|(distance, index)| {
                (
                    distance,
                    if index < self.nodes.len() {
                        self.nodes[index].vantage_point.clone()
                    } else {
                        self.leaves[index - self.nodes.len()].clone()
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
    fn nearest_neigbor_search() {
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

        let expected = Some((13.453624, (60.0, 61.0)));
        let actual = tree.find_nearest_neighbor(&(69.0, 71.0));
        assert_eq!(actual, expected);

        let expected = vec![(4.2426405, (91.0, 16.0)), (13.038404, (95.0, 32.0))];
        let actual = tree.find_k_nearest_neighbors(&(94.0, 19.0), 2);
        assert_eq!(actual, expected);

        let actual = tree.find_neighbors_within_radius(&(94.0, 19.0), 13.038404);
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
        let actual = tree.find_k_nearest_neighbors(&(7.0, 61.0), 10);
        assert_eq!(actual, expected);

        let actual = tree.find_neighbors_within_radius(&(7.0, 61.0), 20.396078);
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
        let actual = tree.find_k_nearest_neighbors(&(84.0, 54.0), 50);
        assert_eq!(actual, expected);

        let actual = tree.find_neighbors_within_radius(&(84.0, 54.0), 92.64988);
        assert_eq!(actual, expected);
    }
    #[test]
    fn utility_functions() {
        let points = vec![(2.0, 3.0), (0.0, 1.0), (4.0, 5.0)];
        let mut tree = VPTree::new(&points, |a, b| {
            ((a.0 - b.0 as f32).powi(2) + (a.1 - b.1 as f32).powi(2)).sqrt()
        });
        assert_eq!(tree.len(), 3);
        tree.insert((9.0, 8.0));
        assert_eq!(tree.len(), 4);
        tree.extend(vec![(19.0, 81.0), (66.0, 36.0)]);
        assert_eq!(tree.len(), 6);
    }
    #[test]
    fn tiny_tree() {
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
        let tree = VPTree::new(&points[0..3], |a, b| {
            ((a.0 - b.0 as f32).powi(2) + (a.1 - b.1 as f32).powi(2)).sqrt()
        });

        let expected = Some((92.63369, (4.0, 5.0)));
        let actual = tree.find_nearest_neighbor(&(69.0, 71.0));
        assert_eq!(actual, expected);

        let expected = vec![(91.08238, (4.0, 5.0)), (93.38094, (2.0, 3.0))];
        let actual = tree.find_k_nearest_neighbors(&(94.0, 19.0), 2);
        assert_eq!(actual, expected);

        let tree = VPTree::new(&points[0..2], |a, b| {
            ((a.0 - b.0 as f32).powi(2) + (a.1 - b.1 as f32).powi(2)).sqrt()
        });

        let expected = Some((95.462036, (2.0, 3.0)));
        let actual = tree.find_nearest_neighbor(&(69.0, 71.0));
        assert_eq!(actual, expected);

        let expected = vec![(93.38094, (2.0, 3.0)), (95.707886, (0.0, 1.0))];
        let actual = tree.find_k_nearest_neighbors(&(94.0, 19.0), 2);
        assert_eq!(actual, expected);

        let tree = VPTree::new(&points[0..1], |a, b| {
            ((a.0 - b.0 as f32).powi(2) + (a.1 - b.1 as f32).powi(2)).sqrt()
        });

        let expected = Some((95.462036, (2.0, 3.0)));
        let actual = tree.find_nearest_neighbor(&(69.0, 71.0));
        assert_eq!(actual, expected);

        let expected = vec![(93.38094, (2.0, 3.0))];
        let actual = tree.find_k_nearest_neighbors(&(94.0, 19.0), 2);
        assert_eq!(actual, expected);

        let tree = VPTree::new(&points[0..0], |a, b| {
            ((a.0 - b.0 as f32).powi(2) + (a.1 - b.1 as f32).powi(2)).sqrt()
        });

        let expected = None;
        let actual = tree.find_nearest_neighbor(&(69.0, 71.0));
        assert_eq!(actual, expected);

        let expected = vec![];
        let actual = tree.find_k_nearest_neighbors(&(94.0, 19.0), 2);
        assert_eq!(actual, expected);
    }
}
