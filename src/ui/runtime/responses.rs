use super::*;
impl UiRuntime {
    pub(super) fn select_response(&mut self, response: SelectResponse) -> EventResult {
        if response.repaint {
            self.invalidation.repaint_semantics();
        }
        if let Some(id) = response.focus {
            self.interaction.focus_keyboard(id);
        }
        if let Some(message) = response.change {
            self.send(message);
        }
        EventResult {
            handled: response.handled,
            repaint: self.invalidation.pending(),
            capture: response.capture,
            release: response.release,
            clipboard: None,
        }
    }

    /// Suite d'un événement du menu contextuel d'un champ : une entrée choisie devient la
    /// commande d'édition du champ (et sa requête de presse-papiers).
    pub(super) fn menu_response(&mut self, response: MenuResponse) -> EventResult {
        if response.repaint {
            self.invalidation.repaint_semantics();
        }
        let mut result = EventResult {
            handled: response.handled,
            repaint: self.invalidation.pending(),
            capture: response.capture,
            release: response.release,
            clipboard: None,
        };
        if let Some(command) = response.command {
            let reply = self.event_inner(UiEvent::Edit(command));
            result.repaint |= reply.repaint;
            result.release |= reply.release;
            result.clipboard = reply.clipboard;
        }
        result
    }

    pub(super) fn slider_response(&mut self, response: SliderResponse) -> EventResult {
        if response.repaint {
            self.invalidation.repaint_semantics();
        }
        if let Some((id, value)) = response.change
            && let Some(message) = self.slider_message(id, value, false)
        {
            self.send(message);
        }
        if let Some((id, value)) = response.commit
            && let Some(message) = self.slider_message(id, value, true)
        {
            self.send(message);
        }
        EventResult {
            handled: response.handled,
            repaint: self.invalidation.pending(),
            capture: response.capture,
            release: response.release,
            clipboard: None,
        }
    }

    /// Gestionnaire lu dans l'arbre courant : un slider retiré n'émet plus rien.
    fn slider_message(&self, id: ElementId, value: f64, commit: bool) -> Option<Message> {
        let slider = self.root.as_ref()?.find(id)?.widget.as_ref()?.slider()?;
        let handler = if commit {
            slider.on_commit.as_ref()?
        } else {
            &slider.on_change
        };
        Some(handler.emit(value))
    }
}
