# icfpc2021
λ-llama code for ICFP contest 2021


## Solution

We used a hybrid approach to solve the problem utilizing three different approaches described
below.

### UI

### Annealing

This was the first algorithmic solution based on local search technique called
simulated annealing. The algorithm incrementaly tries to improve the starting configuration
by applying a series of local moves like moving a single vertex, moving a whole pose, etc.
At every step we estimate how many constraints violations and dislikes the current configuration
has and select the moves in a way that minimizes an score function based on the weighted
combination of these factors.

This solution achieves approximate solutions for many tests in the first half but struggles to
find optimal solutions.

Localted in src/solver/annealing.rs

### Tree Search

This solution is based on the bruteforce search over all integer points configuration with
heuristics to prune the search space. The algorithm selects some order for vertex placement
based on the depth first search traversal and then places vertices one by one in that order.
After the placement of each vertex we narrow down the possible places where future vertices
directly connected to this one can be placed. The search will be able to find all valid
configurations and thus was well suitable to find optimal solutions to many of the smaller
problems in the first half, but struggled to work with more than 50 vertices in the pose.

Localted in src/solver/tree_search.rs
