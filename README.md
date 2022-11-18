# vptree
A vantage-point tree library written in Rust. Implements a nearest-neighbor search, k-nearest-neighbors search and neighbors-within-distance search.

Optimized for memory constrained systems, no child/parent pointer is allocated per node, position of which is instead determined algorithmically.
The downside of this approach is the need to rebuild the tree before searching if nodes have been changed.
However, the tree building process is very quick, as is searching.

A vantage-point tree is a data structure that allows for nearest neighbor search in logarithmic time in non-euclidean metric spaces.
An example use-case would be searching for neighboring numbers by their hamming distance. For a neat, visual explanation see [here](https://fribbels.github.io/vptree/writeup).
