// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying LICENSE file for details.

use std::mem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node<T> {
    data: T,
    children: Vec<Node<T>>,
}

impl<T> Node<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            children: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn add_child(&mut self, child: Node<T>) {
        self.children.push(child);
    }

    pub fn zipper(self) -> Zipper<T> {
        Zipper {
            node: self,
            parent: None,
        }
    }
}

impl<T> AsRef<T> for Node<T> {
    fn as_ref(&self) -> &T {
        &self.data
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParentStatus {
    AtRoot,
    Relocated,
}

#[derive(Debug, Clone)]
pub struct Zipper<T> {
    node: Node<T>,
    parent: Option<(Box<Zipper<T>>, usize)>,
}

impl<T> Zipper<T> {
    pub fn new(node: Node<T>) -> Self {
        Self {
            node,
            parent: None,
        }
    }

    pub fn iter(self) -> ZipperIter<T> {
        ZipperIter::new(self)
    }

    fn child(&mut self, idx: usize) {
        // Find the child.
        let child = self.node.children.swap_remove(idx);
        // Create a new zipper with the child node.
        let child_zipper = Self::new(child);
        // Replace ourself with the new zipper.
        let old_zipper = mem::replace(self, child_zipper);
        // Add the parent information into the child zipper.
        self.parent = Some((Box::new(old_zipper), idx));
    }

    fn parent(&mut self) -> ParentStatus {
        // Extract our parent's information.
        let (mut parent, idx) = if let Some(parent_info) = mem::replace(&mut self.parent, None) {
            parent_info
        } else {
            // We're at the root; nowhere to go.
            return ParentStatus::AtRoot;
        };

        // Swap the node with the parent node.
        mem::swap(&mut self.node, &mut parent.node);

        // Push the old child node back into its position.
        self.node.children.push(parent.node);
        let len = self.node.children.len();
        self.node.children.swap(idx, len - 1);

        // Indicate that we've moved our location.
        ParentStatus::Relocated
    }
}

impl<T> AsRef<T> for Zipper<T> {
    fn as_ref(&self) -> &T {
        self.node.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct ZipperIter<T> {
    zipper: Zipper<T>,
    started: bool,
    done: bool,
}

impl<T> ZipperIter<T> {
    fn new(zipper: Zipper<T>) -> Self {
        Self {
            zipper,
            started: false,
            done: false,
        }
    }

    pub fn add_child(&mut self, node: Node<T>) {
        self.zipper.node.add_child(node)
    }

    pub fn current(&self) -> Option<&T> {
        if self.done {
            return None;
        }

        Some(&self.zipper.node.data)
    }

    pub fn current_mut(&mut self) -> Option<&mut Node<T>> {
        if self.done {
            return None;
        }

        Some(&mut self.zipper.node)
    }

    pub fn next(&mut self) -> Option<&T> {
        if self.done {
            return None;
        }

        if self.started {
            if self.zipper.node.is_empty() {
                // Find the next sibling to use.
                loop {
                    // Find out where to move in the parent.
                    let next_idx = if let Some((_, idx)) = &self.zipper.parent {
                        idx + 1
                    } else {
                        // We've handled this node and it doesn't have a parent; it is over.
                        self.done = true;
                        return None;
                    };

                    // Move to the parent.
                    self.zipper.parent();

                    // If the next sibling index is valid, move to it.
                    if next_idx < self.zipper.node.len() {
                        self.zipper.child(next_idx);
                        break;
                    }

                    // Otherwise loop and find the next sibling.
                }
            } else {
                // Move to the child of the current node.
                self.zipper.child(0);
            }
        } else {
            self.started = true;
        }

        Some(&self.zipper.node.data)
    }
}

#[cfg(test)]
mod test {
    use super::Node;

    #[test]
    fn test_zipper_iter() {
        let tree = Node::new(0);
        let zipper = tree.zipper();
        let mut iter = zipper.iter();
        assert_eq!(iter.next(), Some(&0));
        assert!(!iter.done);
        assert_eq!(iter.next(), None);
        assert!(iter.done);
    }

    #[test]
    fn test_zipper_children() {
        let mut tree = Node::new(0);
        tree.add_child(Node::new(1));
        tree.add_child(Node::new(2));
        tree.add_child(Node::new(3));
        let zipper = tree.zipper();
        let mut iter = zipper.iter();
        assert_eq!(iter.next(), Some(&0));
        assert!(!iter.done);
        assert_eq!(iter.next(), Some(&1));
        assert!(!iter.done);
        assert_eq!(iter.next(), Some(&2));
        assert!(!iter.done);
        assert_eq!(iter.next(), Some(&3));
        assert!(!iter.done);
        assert_eq!(iter.next(), None);
        assert!(iter.done);
    }

    #[test]
    fn test_zipper_siblings() {
        let mut tree = Node::new(0);
        let mut child = Node::new(1);
        child.add_child(Node::new(2));
        child.add_child(Node::new(3));
        tree.add_child(child);
        let zipper = tree.zipper();
        let mut iter = zipper.iter();
        assert_eq!(iter.next(), Some(&0));
        assert!(!iter.done);
        assert_eq!(iter.next(), Some(&1));
        assert!(!iter.done);
        assert_eq!(iter.next(), Some(&2));
        assert!(!iter.done);
        assert_eq!(iter.next(), Some(&3));
        assert!(!iter.done);
        assert_eq!(iter.next(), None);
        assert!(iter.done);
    }
}
