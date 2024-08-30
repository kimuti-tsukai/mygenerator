use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

/// Yield value
/// Input the argument required in `Generator::new()` to c
pub fn yield_<T>(c: &(Receiver<()>, Sender<T>), value: T) {
    while c.0.recv().is_err() {}
    c.1.send(value).unwrap();
}

/// Construct with `Generator::new()`
/// Get value with `next` in Iterator
/// ```
/// use mygenerator::{Generator, yield_};
///
/// let mut g = Generator::new(|c| {
///     yield_(c, "Hello");
///     yield_(c, "World");
/// });
///
/// assert_eq!(g.next(), Some("Hello"));
/// assert_eq!(g.next(), Some("World"));
/// assert_eq!(g.next(), None);
/// ```
#[derive(Debug)]
pub struct Generator<T> {
    co_routine: Option<JoinHandle<()>>,
    sign: Sender<()>,
    yields: Receiver<T>,
}

impl<T> Generator<T> {
    fn yield_sign(&self) {
        if let Some(ref r) = self.co_routine {
            if !r.is_finished() {
                self.sign.send(()).unwrap();
            }
        }
    }
}

impl<T: Send + 'static> Generator<T> {
    /// Constructor of Generator
    /// The argument of closure is channels required to yield
    /// ```
    /// use mygenerator::{Generator, yield_};
    ///
    /// Generator::new(|c| yield_(c, ()));
    /// ```
    pub fn new<F: FnOnce(&(Receiver<()>, Sender<T>)) + Send + 'static>(func: F) -> Self {
        let s: (Sender<()>, Receiver<()>) = mpsc::channel();
        let y: (Sender<T>, Receiver<T>) = mpsc::channel();
        Self {
            co_routine: Some(thread::spawn(|| func(&(s.1, y.0)))),
            sign: s.0,
            yields: y.1,
        }
    }
}

impl<T> Iterator for Generator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let r: &JoinHandle<()> = self.co_routine.as_ref()?;

        self.yield_sign();

        loop {
            if !r.is_finished() {
                if let Ok(v) = self.yields.recv() {
                    break Some(v);
                }
            } else {
                self.co_routine.take().unwrap().join().unwrap();
                break None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn yield_test() {
        let mut g: Generator<&str> = Generator::new(|c| {
            thread::sleep(std::time::Duration::from_secs(1));
            yield_(c, "1 sec");
            thread::sleep(std::time::Duration::from_millis(500));
            yield_(c, "1.5 sec");
        });

        assert_eq!(g.next(), Some("1 sec"));
        assert_eq!(g.next(), Some("1.5 sec"));

        let mut g = Generator::new(|c| {
            yield_(c, "Hello");
            yield_(c, "World");
        });

        assert_eq!(g.next(), Some("Hello"));
        assert_eq!(g.next(), Some("World"));
        assert_eq!(g.next(), None);
    }
}
