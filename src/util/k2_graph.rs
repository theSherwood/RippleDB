extern crate bitvec;

use std::collections::HashMap;

use super::{Triple, CRUD};

/* Subjects and Objects are mapped in the same
     collection to a unique int while Predicates
     are mapped seperately to unique ints.
   Each slice contains a 2-d bit matrix, each cell
     corresponding to a Subject-Object pair connected
     by a single Predicate. */
struct Graph {
  dict_max: usize, //The max value in dict, as it's non-trivial to calculate on the fly
  dict: HashMap<String, usize>,
  predicates: HashMap<String, usize>,
  slices: Vec<Box<k2_tree::K2Tree>>,
}
impl CRUD for Graph {
  type IN = Triple;
  type OUT = ();
  type QUERY = ();
  fn new() -> Self {
    Graph {
      dict_max: 0,
      dict: HashMap::new(),
      predicates: HashMap::new(),
      slices: Vec::new(),
    }
  }
  fn insert(&mut self, val: Self::IN) -> Result<(), ()> {
    let graph_coords = (
      self.dict.get(&val[0]),
      self.predicates.get(&val[1]),
      self.dict.get(&val[2])
    );
    match graph_coords {
      (Some(col), Some(slice_index), Some(row)) => {
        if let Some(slice) = self.slices.get_mut(*slice_index) {
          slice.set_bit(*col, *row, true)
        }
        else {
          Err(())
        }
      },
      (None, Some(slice_index), Some(row)) => {
        /* Insert new subject into hashmap and give it the index of current_max+1
           If the new max is greater than the matrix_widths of slices
           | For each slice:
           | | matrix_width *= k
           | | Insert a new stem of length k**2 at index 0, first bit set to 1
           | | Update the layer_start to be +k**2 each
           Set the correct bit in the correct slice to 1*/
        Err(())
      },
      (Some(col), None, Some(row)) => {
        /* Append a new slice to self.slices, insert to self.predicates
           New slice must be matrix width of all the others and have the same K-value
           new_slice.set_bit(col, row) */
        Err(())
      },
      (Some(col), Some(slice_index), None) => Err(()),
      (None, None, Some(row)) => Err(()),
      (None, Some(slice_index), None) => Err(()),
      (Some(col), None, None) => Err(()),
      (None, None, None) => Err(()),
    }
  }
  fn remove(&mut self, val: &Self::IN) -> Result<(), ()> {
    unimplemented!()
  }
  fn get(&self, query: &Self::QUERY) -> Self::OUT {
    unimplemented!()
  }
}

pub mod k2_tree {
  use bitvec::{
    prelude::bitvec,
    vec::BitVec
  };
  #[derive(Debug, Clone, PartialEq)]
  pub struct K2Tree {
    matrix_width: usize,
    k: usize, //k^2 == number of submatrices in each stem/leaf
    layer_starts: Vec<usize>,
    stems: BitVec,
    stem_to_leaf: Vec<usize>,
    leaves: BitVec,
  }
  /* Public Interface */
  impl K2Tree {
    pub fn new(k: usize) -> Self {
      K2Tree {
        matrix_width: k.pow(3),
        k: k,
        layer_starts: vec![0],
        stems: bitvec![0; k*k],
        stem_to_leaf: Vec::new(),
        leaves: BitVec::new(),
      }
    }
    pub fn test_tree() -> Self {
      K2Tree {
        matrix_width: 8,
        k: 2,
        layer_starts: vec![0, 4],
        stems:  bitvec![0,1,1,1, 1,1,0,1, 1,0,0,0, 1,0,0,0],
        stem_to_leaf: vec![0, 1, 3, 4, 8],
        leaves: bitvec![0,1,1,0, 0,1,0,1, 1,1,0,0, 1,0,0,0, 0,1,1,0],
      }
    }
    pub fn get_bit(&self, x: usize, y: usize) -> bool {
      /* Assuming k=2 */
      if let DescendResult::Leaf(leaf_start, leaf_range) = self.matrix_bit(x, y, self.matrix_width) {
        if leaf_range[0][1] - leaf_range[0][0] != 1
        || leaf_range[0][1] - leaf_range[0][0] != 1 {
          /* ERROR: Final submatrix isn't a 2 by 2 so can't be a leaf */
        }
        if x == leaf_range[0][0] {
          if y == leaf_range[1][0] { return self.leaves[leaf_start] }
          else { return self.leaves[leaf_start+2] }
        }
        else {
          if y == leaf_range[1][0] { return self.leaves[leaf_start+1] }
          else { return self.leaves[leaf_start+3] }
        }
      }
      else {
        /* DescendResult::Stem means no leaf with bit at (x, y) exists,
             so bit must be 0 */
        return false
      }
    }
    pub fn set_bit(&mut self, x: usize, y: usize, state: bool) -> Result<(), ()> {
      /* Assuming k=2 */
      match self.matrix_bit(x, y, self.matrix_width) {
        DescendResult::Leaf(leaf_start, leaf_range) => {
          if leaf_range[0][1] - leaf_range[0][0] != 1
          || leaf_range[0][1] - leaf_range[0][0] != 1 {
            /* ERROR: Final submatrix isn't a 2 by 2 so can't be a leaf */
            return Err(())
          }
          /* Set the bit in the leaf to the new state */
          if x == leaf_range[0][0] {
            if y == leaf_range[1][0] { self.leaves.set(leaf_start, state); }
            else { self.leaves.set(leaf_start+2, state); }
          }
          else {
            if y == leaf_range[1][0] { self.leaves.set(leaf_start+1, state); }
            else { self.leaves.set(leaf_start+3, state); }
          }
          /* If leaf is now all 0's, remove leaf and alter rest of struct to reflect changes.
          Loop up the stems changing the parent bits to 0's and removing stems that become all 0's */
          if ones_in_range(&self.leaves, leaf_start, leaf_start+3) == 0 {
            /* - Remove the leaf
               - Use stem_to_leaf to find the dead leaf's parent bit
               - Remove the elem from stem_to_leaf that mapped to dead leaf
               - Set parent bit to 0, check if stem now all 0's
               - If all 0's:
               - - Remove stem
               - - Alter layer_starts if needed
               - - Find parent bit and set to 0
               - - Repeat until reach stem that isn't all 0's or reach stem layer 0 */
            remove_block(&mut self.leaves, leaf_start, 4)?;
            let stem_bit_pos = self.stem_to_leaf[leaf_start/4];
            self.stem_to_leaf.remove(leaf_start/4);
            if self.stem_to_leaf.len() == 0 {
              /* If no more leaves, then remove all stems immediately
              and don't bother with complex stuff below */
              self.stems = bitvec![0,0,0,0];
              self.layer_starts = vec![0];
              return Ok(())
            }
            let layer_start = self.layer_starts[self.layer_starts.len()-1];
            self.stems.set(layer_start + stem_bit_pos, false); //Dead leaf parent bit = 0
            let mut curr_layer = self.layer_starts.len()-1;
            let mut stem_start = layer_start + block_start(stem_bit_pos, 4);
            while curr_layer > 0 
            && ones_in_range(&self.stems, stem_start, stem_start+3) == 0 {
              for layer_start in &mut self.layer_starts[curr_layer+1..] {
                *layer_start -= 1; //Adjust lower layer start positions to reflect removal of stem
              }
              let (parent_stem_start, bit_offset) = self.parent(stem_start);
              remove_block(&mut self.stems, stem_start, 4)?;
              self.stems.set(parent_stem_start + bit_offset, false);
              stem_start = parent_stem_start;
              curr_layer -= 1;
            }
          }
        },
        DescendResult::Stem(mut stem_start, mut stem_range) if state => {
          /* Descend returning Stem means no Leaf containing bit at (x, y),
          must be located in a submatrix of all 0's.
          If state = false: do nothing 
          If state = true:
           - Construct needed stems until reach final layer
           - Construct leaf corresponding to range containing (x, y)
           - Set bit at (x, y) to 1 */
          let mut layer = self.layer_from_range(stem_range);
          let layer_max = self.total_layers()-2; //Leaf layer doesn't count
          let layer_starts_len = self.layer_starts.len();
          /* If Stem at last layer already exists: skip straight to creating leaf */
          if layer == layer_max {
            let subranges = to_4_subranges(stem_range);
            for child_pos in 0..4 {
              if within_range(&subranges[child_pos], x, y) {
                self.stems.set(stem_start + child_pos, true);
                let layer_bit_pos = (stem_start + child_pos) - self.layer_starts[self.layer_starts.len()-1];
                /* Find position to insert new elem in stem_to_leaf */
                let mut stem_to_leaf_pos: usize = 0;
                while stem_to_leaf_pos < self.stem_to_leaf.len()
                && self.stem_to_leaf[stem_to_leaf_pos] < layer_bit_pos {
                  stem_to_leaf_pos += 1;
                }
                self.stem_to_leaf.insert(stem_to_leaf_pos, layer_bit_pos);
                /* Create new leaf of all 0's */
                let leaf_start = stem_to_leaf_pos * 4;
                insert_block(&mut self.leaves, leaf_start, 4)?;
                /* Change bit at (x, y) to 1 */
                let leaf_range = subranges[child_pos];
                if x == leaf_range[0][0] {
                  if y == leaf_range[1][0] { self.leaves.set(leaf_start, true); }
                  else { self.leaves.set(leaf_start+2, true); }
                }
                else {
                  if y == leaf_range[1][0] { self.leaves.set(leaf_start+1, true); }
                  else { self.leaves.set(leaf_start+3, true); }
                }
                return Ok(())
              }
            }
          }
          while layer <= layer_max {
            let subranges = to_4_subranges(stem_range);
            for child_pos in 0..4 {
              if within_range(&subranges[child_pos], x, y) {
                if layer == layer_max {
                  /* We've reached the final stem layer and this is the final loop iteration,
                  add the stem, add the leaf, link the two together then change the
                  bit at (x, y) to 1 */
                  self.stems.set(stem_start + child_pos, true);
                  let layer_bit_pos = (stem_start + child_pos) - self.layer_starts[self.layer_starts.len()-1];
                  /* Find position to insert new elem in stem_to_leaf */
                  let mut stem_to_leaf_pos: usize = 0;
                  while stem_to_leaf_pos < self.stem_to_leaf.len()
                  && self.stem_to_leaf[stem_to_leaf_pos] < layer_bit_pos {
                    stem_to_leaf_pos += 1;
                  }
                  self.stem_to_leaf.insert(stem_to_leaf_pos, layer_bit_pos);
                  /* Added a stem block in final layer, increase bit positions in stem_to_leaf
                  after the new elem by 4 to stay accurate */
                  let stem_to_leaf_len = self.stem_to_leaf.len();
                  for parent_bit_pos in &mut self.stem_to_leaf[stem_to_leaf_pos+1..stem_to_leaf_len] {
                    *parent_bit_pos += 4;
                  }
                  /* Create new leaf of all 0's */
                  let leaf_start = stem_to_leaf_pos * 4;
                  insert_block(&mut self.leaves, leaf_start, 4)?;
                  let leaf_range = subranges[child_pos];
                  /* Change bit at (x, y) to 1 */
                  if x == leaf_range[0][0] {
                    if y == leaf_range[1][0] { self.leaves.set(leaf_start, true); }
                    else { self.leaves.set(leaf_start+2, true); }
                  }
                  else {
                    if y == leaf_range[1][0] { self.leaves.set(leaf_start+1, true); }
                    else { self.leaves.set(leaf_start+3, true); }
                  }
                  return Ok(())
                }
                else {
                  /* - Change bit containing (x, y) to 1
                     - Get the start position of where the child stem
                       should be and create new stem of 0s there
                     - Update the layer_starts */
                  self.stems.set(stem_start + child_pos, true);
                  if layer == layer_starts_len-1 {
                    stem_start = self.stems.len();
                    self.layer_starts.push(stem_start);
                  }
                  else {
                    stem_start = self.child_stem(layer, stem_start, child_pos)?;
                  }
                  insert_block(&mut self.stems, stem_start, 4)?;
                  if layer+2 <= layer_starts_len {
                    for layer_start in &mut self.layer_starts[layer+2..layer_starts_len] {
                      *layer_start += 4;
                    }
                  }
                  stem_range = subranges[child_pos];
                  break
                }
              }
            }
            layer += 1;
          }
        }
        DescendResult::Nothing => return Err(()), //Descend didn't stop at stem or leaf, should be impossible
        _ => {},
      }
      Ok(())
    }
    pub fn from_matrix(m: Vec<BitVec>) -> Result<Self, ()> {
      let mut tree = K2Tree::new(2);
      for x in 0..m.len() {
        for y in one_positions(&m[x]).into_iter() {
          tree.set_bit(x, y, true)?;
        }
      }
      Ok(tree)
    }
  }
  /* Utils */
  type Range = [[usize; 2]; 2];
  enum DescendResult {
    Leaf(usize, Range), //leaf_start, leaf_range
    Stem(usize, Range), //stem_start, stem_range
    Nothing,
  }
  struct DescendEnv {
    /* Allows for descend to be recursive without parameter hell */
    x: usize,
    y: usize,
    stem_layer_max: usize,
  }
  impl K2Tree {
    fn total_layers(&self) -> usize {
      (self.matrix_width as f64).log(self.k as f64) as usize
    }
    fn layer_from_range(&self, r: Range) -> usize {
      let r_width = r[0][1]-r[0][0]+1;
      ((self.matrix_width as f64).log(self.k as f64) as usize)
      - ((r_width as f64).log(self.k as f64) as usize)
    }
    fn matrix_bit(&self, x: usize, y: usize, m_width: usize) -> DescendResult {
      let env = DescendEnv {
        x: x,
        y: y,
        stem_layer_max: self.layer_starts.len()-1,
      };
      self.descend(&env, 0, 0, [[0, m_width-1], [0, m_width-1]])
    }
    fn descend(&self, env: &DescendEnv, layer: usize, stem_pos: usize, range: Range) -> DescendResult {
      let subranges = to_4_subranges(range);
      for (child_pos, child) in self.stems[stem_pos..stem_pos+4].iter().enumerate() {
        if within_range(&subranges[child_pos], env.x, env.y) {
          if !child { return DescendResult::Stem(stem_pos, range) } //The bit exists within a range that has all zeros
          else if layer == env.stem_layer_max {
            return DescendResult::Leaf(self.leaf_start(stem_pos + child_pos).unwrap(), subranges[child_pos])
          }
          else {
            return self.descend(env,
                                layer+1,
                                self.child_stem(layer, stem_pos, child_pos).unwrap(),
                                subranges[child_pos])
          }
        }
      }
      DescendResult::Nothing //Should never return this but need to satisfy compiler
    }
    fn num_stems_before_child(&self, bit_pos: usize, layer: usize) -> usize {
      let layer_start = self.layer_start(layer);
      ones_in_range(&self.stems, layer_start, bit_pos)
    }
    fn layer_start(&self, l: usize) -> usize {
      if l == self.layer_starts.len() {
        self.stems.len()
      }
      else {
        self.layer_starts[l]
      }
    }
    fn layer_len(&self, l: usize) -> usize {
      if l == self.layer_starts.len()-1 {
        return self.stems.len() - self.layer_starts[l]
      }
      self.layer_starts[l+1] - self.layer_starts[l]
    }
    fn leaf_start(&self, stem_bitpos: usize) -> Result<usize, ()> {
      if !self.stems[stem_bitpos] { return Err(()) }
      Ok(self.stem_to_leaf
             .iter()
             .position(|&n| n == (stem_bitpos - self.layer_starts[self.layer_starts.len()-1]))
             .unwrap()
             * 4)
    }
    fn child_stem(&self, layer: usize, stem_start: usize, nth_child: usize) -> Result<usize, ()> {
      if !self.stems[stem_start+nth_child]
      || layer == self.total_layers()-2 {
        /* If stem_bit is 0 or final stem layer, cannot have children */
        return Err(())
      }
      Ok(self.layer_start(layer+1)
      + (self.num_stems_before_child(stem_start+nth_child, layer) * 4))
    }
    fn parent(&self, stem_start: usize) -> (usize, usize) {
      /* Returns (stem_start, bit_offset) */
      if stem_start >= self.layer_starts[1] {
        /* If stem isn't in layer 0, look for parent */
        let mut parent_layer_start = 0;
        let mut curr_layer_start = 0;
        for (i, layer_start) in (0..self.layer_starts.len()).enumerate() {
          if i == self.layer_starts.len()-1 {
            if stem_start >= self.layer_starts[layer_start]
            && stem_start < self.stems.len() {
              parent_layer_start = self.layer_starts[layer_start-1];
              curr_layer_start = self.layer_starts[layer_start];
            }
          }
          else if stem_start >= self.layer_starts[layer_start]
          && stem_start < self.layer_starts[layer_start+1] {
            parent_layer_start = self.layer_starts[layer_start-1];
            curr_layer_start = self.layer_starts[layer_start];
          }
        }
        let nth_stem_in_layer = (stem_start - curr_layer_start)/4;
        let mut i = 0;
        let mut bit_pos_in_parent_stem_layer = 0;
        for bit in &self.stems[parent_layer_start..curr_layer_start] {
          if bit {
            if i == nth_stem_in_layer { break }
            i += 1;
          }
          bit_pos_in_parent_stem_layer += 1;
        }
        (((bit_pos_in_parent_stem_layer / 4) * 4) + parent_layer_start,
          bit_pos_in_parent_stem_layer % 4)
      }
      else {
        (std::usize::MAX, std::usize::MAX)
      }
    }
    fn parent_stem(&self, stem_start: usize) -> usize {
      self.parent(stem_start).0
    }
    fn parent_bit(&self, stem_start: usize) -> usize {
      let (stem_start, bit_offset) = self.parent(stem_start);
      stem_start + bit_offset
    }
  }
  fn block_start(bit_pos: usize, block_len: usize) -> usize {
    (bit_pos / block_len) * block_len
  }
  fn remove_block(bit_vec: &mut BitVec, block_start: usize, block_len: usize) -> Result<(), ()> {
    if block_start >= bit_vec.len()
    || block_start % block_len != 0 {
      Err(())
    }
    else {
      Ok(for _ in 0..block_len { bit_vec.remove(block_start); })
    }
  }
  fn insert_block(bit_vec: &mut BitVec, block_start: usize, block_len: usize) -> Result<(), ()> {
    if block_start > bit_vec.len()
    || block_start % block_len != 0 {
      Err(())
    }
    else {
      Ok(for _ in 0..block_len { bit_vec.insert(block_start, false); })
    }
  }
  fn to_4_subranges(r: Range) -> [Range; 4] {
    [
      [[r[0][0], r[0][0]+((r[0][1]-r[0][0])/2)],   [r[1][0], r[1][0]+((r[1][1]-r[1][0])/2)]], //Top left quadrant
      [[r[0][0]+((r[0][1]-r[0][0])/2)+1, r[0][1]], [r[1][0], r[1][0]+((r[1][1]-r[1][0])/2)]], //Top right quadrant
      [[r[0][0], r[0][0]+((r[0][1]-r[0][0])/2)],   [r[1][0]+((r[1][1]-r[1][0])/2)+1, r[1][1]]], //Bottom left quadrant
      [[r[0][0]+((r[0][1]-r[0][0])/2)+1, r[0][1]], [r[1][0]+((r[1][1]-r[1][0])/2)+1, r[1][1]]]  //Bottom right quadrant
    ]
  }
  fn within_range(r: &Range, x: usize, y: usize) -> bool {
    x >= r[0][0] && x <= r[0][1] && y >= r[1][0] && y <= r[1][1]
  }
  fn ones_in_range(bits: &BitVec, begin: usize, end: usize) -> usize {
    bits[begin..end].iter().fold(0, |total, bit| total + bit as usize)
  }
  fn one_positions(bit_vec: &BitVec) -> Vec<usize> {
    bit_vec
    .iter()
    .enumerate()
    .filter_map(
      |(pos, bit)|
      if bit { Some(pos) }
      else   { None })
    .collect()
  }
  /* Unit Tests */
  #[cfg(test)]
  pub mod unit_tests {
    use super::*;
    #[test]
    fn from_matrix_0() {
      let m = vec![
        bitvec![0,0,0,0,1,0,0,0],
        bitvec![0; 8],
        bitvec![0; 8],
        bitvec![0; 8],
        bitvec![0,1,0,0,0,1,0,0],
        bitvec![1,0,0,0,1,0,0,0],
        bitvec![0,0,1,0,0,0,0,0],
        bitvec![1,1,1,0,0,0,0,0],
      ];
      let tree = K2Tree {
        matrix_width: 8,
        k: 2,
        layer_starts: vec![0, 4],
        stems:  bitvec![0,1,1,1, 1,1,0,1, 1,0,0,0, 1,0,0,0],
        stem_to_leaf: vec![0, 1, 3, 4, 8],
        leaves: bitvec![0,1,1,0, 0,1,0,1, 1,1,0,0, 1,0,0,0, 0,1,1,0]
      };
      assert_eq!(tree, K2Tree::from_matrix(m).unwrap());
    }
    #[test]
    fn one_positions_0() {
      let bv = bitvec![0,1,0,1,0,1,0,0,0,1];
      assert_eq!(vec![1,3,5,9], one_positions(&bv));
    }
    #[test]
    fn to_4_subranges_0() {
      let ranges = [[[0, 7], [0, 7]], [[4, 7], [0, 3]], [[8, 15], [8, 15]]];
      let subranges = [
        [[[0, 3], [0, 3]], [[4, 7], [0, 3]], [[0, 3], [4, 7]], [[4, 7], [4, 7]]],
        [[[4, 5], [0, 1]], [[6, 7], [0, 1]], [[4, 5], [2, 3]], [[6, 7], [2, 3]]],
        [[[8, 11], [8, 11]], [[12, 15], [8, 11]], [[8, 11], [12, 15]], [[12, 15], [12, 15]]]
      ];
      for i in 0..ranges.len() {
        assert_eq!(to_4_subranges(ranges[i]), subranges[i]);
      }
    }
    #[test]
    fn within_range_0() {
      let coords = [[0, 0], [5, 6], [87, 2],[5, 5]];
      let ranges = [[[0, 3], [0, 3]], [[0, 7], [0, 7]], [[50, 99], [0, 49]], [[5, 9], [5, 9]]];
      for i in 0..coords.len() {
        assert!(within_range(&ranges[i], coords[i][0], coords[i][1]));
      }
    }
    #[test]
    fn ones_in_range_0() {
      let ranges = [
        bitvec![0,1,1,1,0,0,1,0,1,1,0,0],
        bitvec![0,0,0,0,0,0,1],
        bitvec![0,1,1,1,1,1,1,0,1,0,0,1]
      ];
      let num_ones = [6, 1, 8];
      for i in 0..ranges.len() {
        assert_eq!(ones_in_range(&ranges[i], 0, ranges[i].len()), num_ones[i]);
      }
    }
    #[test]
    fn stem_layer_start_0() {
      let tree = K2Tree::test_tree();
      assert_eq!(tree.layer_start(0), 0);
      assert_eq!(tree.layer_start(1), 4);
    }
    #[test]
    fn stem_layer_len_0() {
      let tree = K2Tree::test_tree();
      assert_eq!(tree.layer_len(0), 4);
      assert_eq!(tree.layer_len(1), 12);
    }
    #[test]
    fn leaf_start_0() {
      let tree = K2Tree::test_tree();
      assert_eq!(tree.leaf_start(4), Ok(0));
      assert_eq!(tree.leaf_start(5), Ok(4));
      assert_eq!(tree.leaf_start(7), Ok(8));
      assert_eq!(tree.leaf_start(8), Ok(12));
      assert_eq!(tree.leaf_start(12), Ok(16));
      assert_eq!(tree.leaf_start(9), Err(()));
    }
    #[test]
    fn child_stem_0() {
      let tree = K2Tree::test_tree();
      assert_eq!(tree.child_stem(0, 0, 0), Err(()));
      assert_eq!(tree.child_stem(0, 0, 1), Ok(4));
      assert_eq!(tree.child_stem(0, 0, 2), Ok(8));
      assert_eq!(tree.child_stem(0, 0, 3), Ok(12));
      assert_eq!(tree.child_stem(1, 4, 0), Err(()));
    }
    #[test]
    fn parent_stem_0() {
      let tree = K2Tree::test_tree();
      assert_eq!(tree.parent_stem(4), 0);
      assert_eq!(tree.parent_stem(8), 0);
      assert_eq!(tree.parent_stem(12), 0);
    }
    #[test]
    fn parent_bit_0() {
      let tree = K2Tree::test_tree();
      assert_eq!(tree.parent_bit(4), 1);
      assert_eq!(tree.parent_bit(8), 2);
      assert_eq!(tree.parent_bit(12), 3);
    }
    #[test]
    fn set_bit_0() {
      let mut tree = K2Tree::test_tree();
      assert_eq!(tree.leaves[18], true);
      tree.set_bit(4, 5, false);
      assert_eq!(tree.leaves[18], false);
    }
    #[test]
    fn set_bit_1() {
      let mut tree = K2Tree::test_tree();
      assert_eq!(tree.stems, bitvec![0,1,1,1,1,1,0,1,1,0,0,0,1,0,0,0]);
      assert_eq!(tree.leaves, bitvec![0,1,1,0,0,1,0,1,1,1,0,0,1,0,0,0,0,1,1,0]);
      assert_eq!(tree.stem_to_leaf, vec![0, 1, 3, 4, 8]);
      tree.set_bit(4, 5, false);
      tree.set_bit(5, 4, false);
      assert_eq!(tree.stems, bitvec![0,1,1,0,1,1,0,1,1,0,0,0]);
      assert_eq!(tree.leaves, bitvec![0,1,1,0,0,1,0,1,1,1,0,0,1,0,0,0]);
      assert_eq!(tree.stem_to_leaf, vec![0, 1, 3, 4]);
    }
    #[test]
    fn set_bit_2() {
      let mut tree = K2Tree::test_tree();
      assert_eq!(tree.stems, bitvec![0,1,1,1,1,1,0,1,1,0,0,0,1,0,0,0]);
      assert_eq!(tree.leaves, bitvec![0,1,1,0,0,1,0,1,1,1,0,0,1,0,0,0,0,1,1,0]);
      assert_eq!(tree.stem_to_leaf, vec![0, 1, 3, 4, 8]);
      tree.set_bit(4, 5, false);
      tree.set_bit(5, 4, false);
      tree.set_bit(0, 0, true);
      assert_eq!(tree.stems, bitvec![1,1,1,0,1,0,0,0,1,1,0,1,1,0,0,0]);
      assert_eq!(tree.leaves, bitvec![1,0,0,0,0,1,1,0,0,1,0,1,1,1,0,0,1,0,0,0]);
      assert_eq!(tree.stem_to_leaf, vec![0, 4, 5, 7, 8]);
    }
    #[test]
    fn show_me_the_changes() {
      let mut tree = K2Tree::test_tree();
      println!("{:#?}", tree);
      tree.set_bit(4, 5, false);
      println!("{:#?}", tree);
      tree.set_bit(5, 4, false);
      println!("{:#?}", tree);
      tree.set_bit(0, 4, false);
      println!("{:#?}", tree);
      tree.set_bit(0, 0, true);
      println!("{:#?}", tree);
      tree.set_bit(0, 1, true);
      println!("{:#?}", tree);
      tree.set_bit(7, 7, true);
      println!("{:#?}", tree);
      tree.set_bit(5, 4, true);
      println!("{:#?}", tree);
    }
  }
}