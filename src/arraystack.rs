use std::fmt::{Debug, Formatter};

use thiserror::Error;

#[derive(Clone, Copy)]
pub struct Stack<T: Copy + Default, const N: usize> {
    stack: [T; N],
    ptr: usize,
}

impl<T: Copy + Default, const N: usize> Stack<T, N> {
    pub fn new() -> Self {
        Self {
            stack: [Default::default(); N],
            ptr: 0,
        }
    }

    pub fn try_push(&mut self, elem: T) -> Result<(), StackOverflowException> {
        if self.ptr < N {
            self.stack[self.ptr] = elem;
            self.ptr += 1;
            Ok(())
        } else {
            Err(StackOverflowException)
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.ptr > 0 {
            self.ptr -= 1;
            Some(self.stack[self.ptr])
        } else {
            None
        }
    }
}

impl<T, const N: usize> Debug for Stack<T, N>
where
    T: Debug + Copy + Default,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(&self.stack[..self.ptr]).finish()
    }
}

#[derive(Debug, Error)]
#[error("Stack overflowed")]
pub struct StackOverflowException;

#[cfg(test)]
mod tests {
    use crate::arraystack::Stack;

    #[test]
    fn push_one_value() {
        let mut stack: Stack<u8, 1> = Stack::new();
        assert!(matches!(stack.try_push(7), Ok(())));
        assert!(matches!(stack.pop(), Some(7)));
    }

    #[test]
    fn push_three_values() {
        let mut stack: Stack<u8, 5> = Stack::new();
        assert!(matches!(stack.try_push(1), Ok(())));
        assert!(matches!(stack.try_push(2), Ok(())));
        assert!(matches!(stack.try_push(3), Ok(())));
        assert!(matches!(stack.pop(), Some(3)));
        assert!(matches!(stack.pop(), Some(2)));
        assert!(matches!(stack.pop(), Some(1)));
    }

    #[test]
    fn overflow_returns_err() {
        let mut stack: Stack<u8, 1> = Stack::new();
        assert!(matches!(stack.try_push(7), Ok(())));
        assert!(matches!(stack.try_push(7), Err(_)));
        assert!(matches!(stack.pop(), Some(7)));
    }

    #[test]
    fn underflow_returns_none() {
        let mut stack: Stack<u8, 1> = Stack::new();
        assert!(matches!(stack.try_push(7), Ok(())));
        assert!(matches!(stack.pop(), Some(7)));
        assert!(matches!(stack.pop(), None));
    }

    #[test]
    fn pop_from_empty_stack_returns_none() {
        let mut stack: Stack<u8, 1> = Stack::new();
        assert!(matches!(stack.pop(), None));
        assert!(matches!(stack.pop(), None));
        assert!(matches!(stack.pop(), None));
    }
}
