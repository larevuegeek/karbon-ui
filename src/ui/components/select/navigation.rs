use super::*;
impl OpenSelect {
    pub(super) fn navigate(&mut self, key: Key) {
        let enabled: Vec<_> = self
            .options
            .iter()
            .filter(|o| o.enabled)
            .map(|o| o.id)
            .collect();
        if enabled.is_empty() {
            self.active = None;
            return;
        }
        let current = self
            .active
            .and_then(|id| enabled.iter().position(|v| *v == id));
        let index = match key {
            Key::Home => 0,
            Key::End => enabled.len() - 1,
            Key::Up => current.map_or(enabled.len() - 1, |i| i.saturating_sub(1)),
            Key::PageUp => current.unwrap_or(0).saturating_sub(self.rows()),
            Key::PageDown => (current.unwrap_or(0) + self.rows()).min(enabled.len() - 1),
            _ => current.map_or(0, |i| (i + 1).min(enabled.len() - 1)),
        };
        self.active = Some(enabled[index]);
        self.query.clear();
        self.typed = None;
        self.reveal();
    }

    pub(super) fn search(&mut self, text: &str) {
        let text: String = text.chars().filter(|c| !c.is_control()).take(64).collect();
        if text.trim().is_empty() {
            return;
        }
        let now = Instant::now();
        let recent = self
            .typed
            .is_some_and(|t| now.duration_since(t) < Duration::from_secs(1));
        let incoming = text.to_lowercase();
        let repeating = recent && self.query == incoming;
        let query = if recent && !repeating {
            format!("{}{}", self.query, incoming)
        } else {
            incoming.clone()
        };
        let start = if repeating {
            self.active
                .and_then(|id| self.options.iter().position(|o| o.id == id))
                .map_or(0, |i| i + 1)
        } else {
            0
        };
        let find = |prefix: &str| {
            (0..self.options.len())
                .map(|i| (i + start) % self.options.len())
                .find(|&i| {
                    self.options[i].enabled
                        && self.options[i].label.to_lowercase().starts_with(prefix)
                })
        };
        let (query, index) = if let Some(i) = find(&query) {
            (query, Some(i))
        } else {
            (incoming.clone(), find(&incoming))
        };
        self.query = query.chars().take(64).collect();
        self.typed = Some(now);
        if let Some(index) = index {
            self.active = Some(self.options[index].id);
            self.reveal();
        }
    }
}
