use super::*;
impl UiRuntime {
    pub(super) fn activate(&mut self, id: ElementId) {
        if self
            .root
            .as_ref()
            .and_then(|r| r.find(id))
            .and_then(|n| n.widget.as_ref())
            .and_then(|w| w.select())
            .is_some_and(|s| s.enabled)
        {
            let response = self.components.selects.toggle(
                id,
                self.root.as_ref().unwrap(),
                self.layer_viewport(),
            );
            self.select_response(response);
            return;
        }
        if self
            .root
            .as_ref()
            .and_then(|r| r.find(id))
            .and_then(|n| n.widget.as_ref())
            .and_then(|w| w.choice())
            == Some(ChoiceState::Radio(true))
        {
            return;
        }
        let close_menu = self
            .root
            .as_ref()
            .and_then(popover::top)
            .filter(|n| {
                let role = popover::config(n).and_then(|p| p.kind.item_role());
                n.find(id)
                    .and_then(|n| n.widget.as_ref())
                    .is_some_and(|w| Some(w.semantics().role) == role)
            })
            .and_then(popover::config)
            .map(|p| p.on_dismiss.emit());
        // Construire le message depuis l'arbre courant avant de rendre la main à l'application.
        let message = self
            .root
            .as_ref()
            .and_then(|r| r.find(id))
            .and_then(|node| {
                if let Some(list) = &node.virtual_list {
                    list.active
                        .and_then(|index| list.indices.iter().position(|&i| i == index))
                        .and_then(|i| virtual_list::first_action(&node.children[i]))
                        .map(|(_, action)| action)
                } else {
                    node.widget.as_ref().and_then(|w| w.action())
                }
                .map(|action| action.emit())
            });
        if let Some(message) = message {
            self.send(message);
        }
        if let Some(message) = close_menu {
            self.send(message);
        }
    }
}
