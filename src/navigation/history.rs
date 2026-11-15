//! Historique de navigation : page courante, pages précédentes, pages suivantes.

/// Back and forward history of an application's pages, like a browser tab's: opening a
/// page drops the pages ahead of the current one.
///
/// `R` is the application's route type (an enum of its pages, usually).
///
/// ```
/// use karbon_ui::navigation::History;
///
/// let mut history = History::new("home");
/// history.push("albums");
/// history.push("album:42");
/// assert!(history.back());
/// assert_eq!(*history.current(), "albums");
/// assert!(history.can_go_forward());
/// history.push("artists"); // the album is no longer ahead
/// assert!(!history.forward());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct History<R> {
    back: Vec<R>,
    current: R,
    forward: Vec<R>,
}

impl<R: PartialEq> History<R> {
    /// History opened on `start`, with nothing behind or ahead.
    pub fn new(start: R) -> Self {
        Self {
            back: Vec::new(),
            current: start,
            forward: Vec::new(),
        }
    }
    /// The page shown.
    pub fn current(&self) -> &R {
        &self.current
    }
    /// Opens `route`: the current page goes behind, the pages ahead are dropped. Returns
    /// `false` (and changes nothing) when `route` is already the current page.
    pub fn push(&mut self, route: R) -> bool {
        if route == self.current {
            return false;
        }
        let previous = std::mem::replace(&mut self.current, route);
        self.back.push(previous);
        self.forward.clear();
        true
    }
    /// Replaces the current page without adding an entry (a redirect, a page whose
    /// parameters changed).
    pub fn replace(&mut self, route: R) {
        self.current = route;
    }
    /// Goes back one page; `false` when there is none.
    pub fn back(&mut self) -> bool {
        let Some(previous) = self.back.pop() else {
            return false;
        };
        let current = std::mem::replace(&mut self.current, previous);
        self.forward.push(current);
        true
    }
    /// Goes forward one page; `false` when there is none.
    pub fn forward(&mut self) -> bool {
        let Some(next) = self.forward.pop() else {
            return false;
        };
        let current = std::mem::replace(&mut self.current, next);
        self.back.push(current);
        true
    }
    /// Goes back to the most recent page that `skip` does not match, dropping the pages
    /// in between (leaving a group of settings pages at once, for example); without
    /// such a page, opens `fallback`. The page left is the only one ahead afterwards.
    ///
    /// ```
    /// use karbon_ui::navigation::History;
    ///
    /// let mut history = History::new("albums");
    /// history.push("settings/general");
    /// history.push("settings/audio");
    /// history.back_past(|r| r.starts_with("settings/"), "home");
    /// assert_eq!(*history.current(), "albums");
    /// assert!(!history.can_go_back());
    /// ```
    pub fn back_past(&mut self, skip: impl Fn(&R) -> bool, fallback: R) -> bool {
        let Some(i) = self.back.iter().rposition(|r| !skip(r)) else {
            return self.push(fallback);
        };
        let target = self.back.remove(i);
        self.back.truncate(i);
        let current = std::mem::replace(&mut self.current, target);
        self.forward.clear();
        self.forward.push(current);
        true
    }
    /// `true` when [`back`](Self::back) has a page to go to.
    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }
    /// `true` when [`forward`](Self::forward) has a page to go to.
    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }
    /// Pages behind the current one, the most recent last.
    pub fn behind(&self) -> &[R] {
        &self.back
    }
    /// Pages ahead of the current one, the next one last.
    pub fn ahead(&self) -> &[R] {
        &self.forward
    }
}
