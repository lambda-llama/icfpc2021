# icfpc2021
λ-llama code for ICFP contest 2021

## Solution

We used a hybrid approach to solve the problem utilizing three different approaches described
below.

### UI

The UI allows solving the problems by hand, including features such as dragging vertices or sets
of vertices (with guidelines for the best fitting positions), zoom/pan, vertex selection with
add/remove to the selected region, adding adjacent and inverting the selection.

The UI also allows performing various transformations on the selected vertices, such as:
  - Pull (for any outgoing long edges shorten them to the default size, bringing vertices near)
  - Push (for any outgoing long edges shorten them to the default size, pushing out the selected vertices and repeating for the other vertices in the selection)
  - Center (forcing the vertices to take the position that minimizes the sum of edge errors)
  - Horizontal/vertical flips
  - Gradual rotation
  - Folding (requires the folding line to separate the graph components, otherwise it will just flip the pose)

Additionally, the selected solver could be run
from the UI (single step or continuously while holding the button), and the problems can be loaded
directly from the UI as well.

Located in `src/render.rs` (UI) and `src/transform.rs` (transformations)

### Annealing

This was the first algorithmic solution based on local search technique called
simulated annealing. The algorithm incrementaly tries to improve the starting configuration
by applying a series of local moves like moving a single vertex, moving a whole pose, etc.
At every step we estimate how many constraints violations and dislikes the current configuration
has and select the moves in a way that minimizes an score function based on the weighted
combination of these factors.

This solution achieves approximate solutions for many tests in the first half but struggles to
find optimal solutions.

Located in `src/solver/annealing.rs`

### Tree Search

This solution is based on the bruteforce search over all integer points configuration with
heuristics to prune the search space. The algorithm selects some order for vertex placement
based on the depth first search traversal and then places vertices one by one in that order.
After the placement of each vertex we narrow down the possible places where future vertices
directly connected to this one can be placed. The search will be able to find all valid
configurations and thus was well suitable to find optimal solutions to many of the smaller
problems in the first half, but struggled to work with more than 50 vertices in the pose.

Located in `src/solver/tree_search.rs`
