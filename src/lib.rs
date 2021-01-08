use num_traits::Bounded;
use std::cmp::Ordering;
use std::collections::HashSet;

const NO_NODE: u32 = u32::max_value();
struct Node<Item> {
    near: u32,
    far: u32,
    vantage_point: Item,
    radius: f32,
    idx: u32,
}

pub struct VPTree<Item, Distance>
where
    Item: Clone,
    Distance: Fn(&Item, &Item) -> f32,
{
    distance_calculator: Distance,
    nodes: Vec<Node<Item>>,
    root: u32,
}

impl<Item, Distance> VPTree<Item, Distance>
where
    Item: Clone,
    Distance: Fn(&Item, &Item) -> f32,
{
    pub fn new(items: &[Item], distance_calculator: Distance) -> Self {
        let mut nodes = Vec::with_capacity(items.len());
        let mut indexes: Vec<_> = (0..items.len() as u32)
            .map(|i| (i, f32::max_value()))
            .collect();

        let root = Self::create_node(&mut indexes[..], &mut nodes, items, &distance_calculator);
        Self {
            distance_calculator,
            nodes,
            root,
        }
    }

    fn create_node(
        indexes: &mut [(u32, f32)],
        nodes: &mut Vec<Node<Item>>,
        items: &[Item],
        distance_calculator: &dyn Fn(&Item, &Item) -> f32,
    ) -> u32 {
        if indexes.len() == 0 {
            return NO_NODE;
        }

        if indexes.len() == 1 {
            let node_idx = nodes.len();
            nodes.push(Node {
                near: NO_NODE,
                far: NO_NODE,
                vantage_point: items[indexes[0].0 as usize].clone(),
                idx: indexes[0].0,
                radius: f32::max_value(),
            });
            return node_idx as u32;
        }

        let last = indexes.len() - 1;
        let ref_idx = indexes[last].0;

        // Removes the `ref_idx` item from remaining items, because it's included in the current node
        let rest = &mut indexes[..last];
        let vantage_point = items[ref_idx as usize].clone();

        for i in rest.iter_mut() {
            i.1 = distance_calculator(&vantage_point, &items[i.0 as usize]);
        }
        rest.sort_unstable_by(|a, b| {
            if a.1 < b.1 {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        });

        // Remaining items are split by the median distance
        let half_idx = rest.len() / 2;

        let (near_indexes, far_indexes) = rest.split_at_mut(half_idx);
        let vantage_point = items[ref_idx as usize].clone();
        let radius = far_indexes[0].1;

        // push first to reserve space before its children
        let node_idx = nodes.len();
        nodes.push(Node {
            vantage_point,
            idx: ref_idx,
            radius,
            near: NO_NODE,
            far: NO_NODE,
        });

        let near = Self::create_node(near_indexes, nodes, items, distance_calculator);
        let far = Self::create_node(far_indexes, nodes, items, distance_calculator);
        nodes[node_idx].near = near;
        nodes[node_idx].far = far;
        node_idx as u32
    }

    fn search_node(
        &self,
        node: &Node<Item>,
        nodes: &[Node<Item>],
        max_item_count: usize,
        mut max_observed_distance: f32,
        distance_x_index: &mut Vec<(f32, u32)>,
        needle: &Item,
    ) {
        let distance = (self.distance_calculator)(needle, &node.vantage_point);
        let candidate_index = node.idx;

        if distance < max_observed_distance || distance_x_index.len() < max_item_count {
            let index = candidate_index;
            // Add the new item at the end of the list.
            distance_x_index.push((distance, index));
            // We only need to sort lists with more than one entry
            if distance_x_index.len() > 1 {
                // Start indexing at the end of the vector. Note that len() is 1 indexed.
                let mut n = distance_x_index.len() - 1;
                // at n is further than n -1 we swap the two.
                // Prefrom a single insertion sort pass. If the distance of the element
                while n > 0 && distance_x_index[n].0 < distance_x_index[n - 1].0 {
                    distance_x_index.swap(n, n - 1);
                    n = n - 1;
                }
                distance_x_index.truncate(max_item_count);
            }
            // Update the max observed distance, unwrap is safe because this function
            // inserts a point and the `max_item_count` is more then 0.
            max_observed_distance = distance_x_index.last().unwrap().0
        }

        // Recurse towards most likely candidate first to narrow best candidate's distance as soon as possible
        if distance < node.radius {
            // No-node case uses out-of-bounds index, so this reuses a safe bounds check as the "null" check
            if let Some(near) = nodes.get(node.near as usize) {
                self.search_node(
                    near,
                    nodes,
                    max_item_count,
                    max_observed_distance,
                    distance_x_index,
                    needle,
                );
            }
            // The best node (final answer) may be just ouside the radius, but not farther than
            // the best distance we know so far. The search_node above should have narrowed
            // best_candidate.distance, so this path is rarely taken.
            if let Some(far) = nodes.get(node.far as usize) {
                if distance + max_observed_distance >= node.radius {
                    self.search_node(
                        far,
                        nodes,
                        max_item_count,
                        max_observed_distance,
                        distance_x_index,
                        needle,
                    );
                }
            }
        } else {
            if let Some(far) = nodes.get(node.far as usize) {
                self.search_node(
                    far,
                    nodes,
                    max_item_count,
                    max_observed_distance,
                    distance_x_index,
                    needle,
                );
            }
            if let Some(near) = nodes.get(node.near as usize) {
                if distance <= node.radius + max_observed_distance {
                    self.search_node(
                        near,
                        nodes,
                        max_item_count,
                        max_observed_distance,
                        distance_x_index,
                        needle,
                    );
                }
            }
        }
    }

    pub fn find_nearest(&self, needle: &Item, count: usize) -> HashSet<u32> {
        let mut results = vec![];
        if let Some(root) = self.nodes.get(self.root as usize) {
            self.search_node(
                root,
                &self.nodes,
                count,
                f32::max_value(),
                &mut results,
                needle,
            );
        }

        results.into_iter().map(|(_, index)| index).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

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

        let expected = [24].iter().cloned().collect::<HashSet<u32>>();
        let actual = tree.find_nearest(&(69.0, 71.0), 1);
        assert_eq!(actual, expected);

        let expected = [50, 7].iter().cloned().collect::<HashSet<u32>>();
        let actual = tree.find_nearest(&(94.0, 19.0), 2);
        assert_eq!(actual, expected);

        let expected = [22, 40, 37, 11, 8, 44, 31, 47, 19, 33]
            .iter()
            .cloned()
            .collect::<HashSet<u32>>();
        let actual = tree.find_nearest(&(7.0, 61.0), 10);
        assert_eq!(actual, expected);

        let expected = [
            43, 34, 7, 39, 51, 42, 46, 18, 6, 16, 9, 47, 48, 52, 35, 49, 31, 21, 32, 40, 44, 13,
            50, 25, 22, 41, 3, 30, 4, 12, 36, 8, 37, 26, 20, 10, 33, 24, 23, 19, 27, 38, 14, 28,
            29, 45, 17, 5, 15, 11,
        ]
        .iter()
        .cloned()
        .collect::<HashSet<u32>>();
        let actual = tree.find_nearest(&(84.0, 54.0), 50);
        assert_eq!(actual, expected);
    }
}
