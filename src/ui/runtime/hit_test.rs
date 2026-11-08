use super::*;
pub(super) type Walk<T> =
    fn(&Element, PointF, &crate::ui::animation::timeline::Timeline, bool) -> Option<T>;
/// `pointer-events` de `node`, hérité de `inherited` s'il n'en déclare pas.
pub(super) fn pointer_events(node: &Element, inherited: bool) -> bool {
    node.styles
        .as_ref()
        .and_then(|s| s.pointer_events)
        .unwrap_or(inherited)
}
pub(super) fn fixed_result<T>(
    node: &Element,
    p: PointF,
    motions: &crate::ui::animation::timeline::Timeline,
    events: bool,
    walk: Walk<T>,
) -> Option<T> {
    let events = pointer_events(node, events);
    for index in layout::paint_order(node).rev() {
        let child = &node.children[index];
        if let Some(hit) = fixed_result(child, p, motions, events, walk) {
            return Some(hit);
        }
        if layout::fixed(child)
            && let Some(hit) = walk(child, p, motions, events)
        {
            return Some(hit);
        }
    }
    None
}
impl UiRuntime {
    /// Survol visuel indépendant des cibles d'action : cartes, textes et padding.
    pub(super) fn motion_hover(&self) -> Option<ElementId> {
        if !self.motions.has_tracks() {
            return None;
        }
        fn walk(
            node: &Element,
            world: PointF,
            motions: &crate::ui::animation::timeline::Timeline,
            events: bool,
        ) -> Option<ElementId> {
            let clip = motions.clip(node);
            if !layout::contains(clip, world.x, world.y) {
                return None;
            }
            let local = motions.point(node.id, world)?;
            if layout::rounded_clip(node)
                .is_some_and(|(r, radii)| !layout::contains_rounded(r, radii, local))
            {
                return None;
            }
            let events = pointer_events(node, events);
            for index in layout::paint_order(node).rev() {
                let child = &node.children[index];
                if layout::fixed(child) {
                    continue;
                }
                if let Some(id) = walk(child, world, motions, events) {
                    return Some(id);
                }
            }
            (events && layout::contains(node.rect, local.x, local.y)).then_some(node.id)
        }
        let root = layers::active(self.root.as_ref()?);
        let p = self.interaction.pointer?;
        fixed_result(root, p, &self.motions, true, walk)
            .or_else(|| walk(root, p, &self.motions, true))
    }
    /// Curseur déclaré par CSS (`cursor`) sous le pointeur : celui de l'élément le plus
    /// profond touché ou de son plus proche ancêtre qui en déclare un ; un champ de
    /// saisie vaut `text`. `None` sans déclaration (curseur des cibles d'action).
    fn css_cursor(&self) -> Option<Cursor> {
        /// `Some(curseur déclaré éventuel)` quand le point touche l'élément.
        fn walk(
            node: &Element,
            world: PointF,
            motions: &crate::ui::animation::timeline::Timeline,
            events: bool,
        ) -> Option<Option<Cursor>> {
            let clip = motions.clip(node);
            if !layout::contains(clip, world.x, world.y) {
                return None;
            }
            let local = motions.point(node.id, world)?;
            if layout::rounded_clip(node)
                .is_some_and(|(r, radii)| !layout::contains_rounded(r, radii, local))
            {
                return None;
            }
            let events = pointer_events(node, events);
            // Curseur résolu (états compris), sinon celui de la base.
            let declared = node.styles.as_ref().and_then(|s| {
                if s.ready {
                    s.computed.local.cursor
                } else {
                    s.normal.cursor
                }
            });
            let own = declared.or_else(|| {
                node.widget
                    .as_ref()
                    .and_then(|w| w.text_input())
                    .map(|_| Cursor::Text)
            });
            for index in layout::paint_order(node).rev() {
                let child = &node.children[index];
                if layout::fixed(child) {
                    continue;
                }
                if let Some(found) = walk(child, world, motions, events) {
                    return Some(found.or(own));
                }
            }
            (events && layout::contains(node.rect, local.x, local.y)).then_some(own)
        }
        let root = layers::active(self.root.as_ref()?);
        let p = self.interaction.pointer?;
        fixed_result(root, p, &self.motions, true, walk)
            .or_else(|| walk(root, p, &self.motions, true))
            .flatten()
    }

    pub(super) fn hit(&self, p: PointF) -> Option<Hit> {
        /// `None` : le point ne touche pas le sous-arbre (le frère peint dessous est
        /// essayé) ; `Some(None)` : il touche un élément sans cible d'action, qui retient
        /// le point comme dans un navigateur (seuls ses ancêtres peuvent encore le
        /// prendre) ; `Some(Some(hit))` : la cible trouvée.
        fn walk(
            node: &Element,
            world: PointF,
            motions: &crate::ui::animation::timeline::Timeline,
            events: bool,
        ) -> Option<Option<Hit>> {
            let clip = motions.clip(node);
            if !layout::contains(clip, world.x, world.y) {
                return None;
            }
            let p = motions.point(node.id, world)?;
            if layout::rounded_clip(node)
                .is_some_and(|(r, radii)| !layout::contains_rounded(r, radii, p))
            {
                return None;
            }
            let events = pointer_events(node, events);
            if events
                && let Some(s) = node.scroll
                && s.max_offset() > 0.
                && layout::contains(s.track, p.x, p.y)
            {
                return Some(Some(if layout::contains(s.thumb, p.x, p.y) {
                    Hit::ScrollThumb(node.id)
                } else {
                    Hit::ScrollTrack(node.id)
                }));
            }
            if events
                && let Layout::Split(split) = node.layout
                && split.resizable
                && node.divider.is_some_and(|r| layout::contains(r, p.x, p.y))
            {
                return Some(Some(Hit::Divider(node.id, split.axis)));
            }
            // Enfants du dernier peint au premier ; `pointer-events: none` : seuls les
            // descendants qui le rétablissent.
            let mut absorbed = false;
            for index in layout::paint_order(node).rev() {
                let child = &node.children[index];
                if layout::fixed(child) {
                    continue;
                }
                match walk(child, world, motions, events) {
                    Some(Some(hit)) => return Some(Some(hit)),
                    Some(None) => {
                        absorbed = true;
                        break;
                    }
                    None => {}
                }
            }
            if !events {
                return absorbed.then_some(None);
            }
            if !absorbed && !super::hover::covers(node, p) {
                return None;
            }
            // Le point est sur la boîte de l'élément, bordure intérieure comprise (un
            // bouton, un champ ou une liste de choix la prennent comme dans un navigateur),
            // ou sur un enfant (le clic remonte à l'ancêtre). Une glissière et une zone de
            // défilement gardent leur boîte de contenu.
            let inside = absorbed || layout::contains(layout::content_rect(node), p.x, p.y);
            let widget = node.widget.as_ref();
            Some(
                if widget.and_then(|w| w.select()).is_some_and(|s| s.enabled) {
                    Some(Hit::Select(node.id))
                } else if inside && widget.and_then(|w| w.slider()).is_some_and(|s| s.enabled) {
                    Some(Hit::Slider(node.id))
                } else if widget
                    .and_then(|w| w.text_input())
                    .is_some_and(|i| i.enabled)
                {
                    Some(Hit::Input(node.id))
                } else if widget.is_some_and(|w| w.action().is_some()) {
                    Some(Hit::Button(node.id))
                } else if inside && node.scroll.is_some() {
                    Some(Hit::ScrollArea(node.id))
                } else {
                    None
                },
            )
        }
        self.root.as_ref().and_then(|root| {
            let scope = layers::active(root);
            // Liste de suggestions au-dessus d'un dialogue : elle d'abord, puis le dialogue.
            popover::top(root)
                .filter(|n| popover::is_combobox(n) && !std::ptr::eq(scope, root))
                .and_then(|list| walk(list, p, &self.motions, true))
                .or_else(|| fixed_result(scope, p, &self.motions, true, walk))
                .or_else(|| walk(scope, p, &self.motions, true))
                .flatten()
        })
    }

    /// `true` when pressing at `p` (window coordinates, logical pixels) should move the
    /// window: the point is inside an element marked with
    /// [`drag_region`](crate::ui::element::Element::drag_region) and not in a part excluded
    /// by [`no_drag`](crate::ui::element::Element::no_drag), no interactive target is under
    /// it, and no menu, popover or dialog is open (a click there closes it).
    pub fn drag_region_at(&self, p: PointF) -> bool {
        let Some(root) = self.root.as_ref() else {
            return false;
        };
        if !std::ptr::eq(layers::active(root), root) || popover::top(root).is_some() {
            return false;
        }
        if self
            .hit(p)
            .is_some_and(|hit| !matches!(hit, Hit::ScrollArea(_)))
        {
            return false;
        }
        // Décision de l'élément le plus profond qui en porte une, comme
        // `-webkit-app-region` : `data-no-drag` retire une partie d'une zone de
        // déplacement, une zone imbriquée dans cette partie déplace de nouveau.
        fn walk(
            node: &Element,
            world: PointF,
            motions: &crate::ui::animation::timeline::Timeline,
        ) -> Option<bool> {
            if layout::hidden(node) || !layout::contains(motions.clip(node), world.x, world.y) {
                return None;
            }
            let local = motions.point(node.id, world)?;
            // Le dernier enfant peint par-dessus les autres : il l'emporte.
            let inner = node
                .children
                .iter()
                .rev()
                .find_map(|child| walk(child, world, motions));
            inner.or(node
                .drag_region
                .filter(|_| layout::contains(node.rect, local.x, local.y)))
        }
        walk(root, p, &self.motions) == Some(true)
    }

    /// Edge of the window to resize when pressing at `p` (window coordinates, logical
    /// pixels): the point is inside an element marked with
    /// [`resize_region`](crate::ui::element::Element::resize_region), no interactive
    /// target is under it, and no menu, popover or dialog is open.
    pub fn resize_region_at(&self, p: PointF) -> Option<crate::window::ResizeEdge> {
        let root = self.root.as_ref()?;
        if !std::ptr::eq(layers::active(root), root) || popover::top(root).is_some() {
            return None;
        }
        if self.hit(p).is_some() {
            return None;
        }
        fn walk(
            node: &Element,
            world: PointF,
            motions: &crate::ui::animation::timeline::Timeline,
        ) -> Option<crate::window::ResizeEdge> {
            if layout::hidden(node) || !layout::contains(motions.clip(node), world.x, world.y) {
                return None;
            }
            let local = motions.point(node.id, world)?;
            // Le dernier enfant peint par-dessus les autres : il l'emporte.
            let inner = node
                .children
                .iter()
                .rev()
                .find_map(|child| walk(child, world, motions));
            inner.or(node
                .resize_region
                .filter(|_| layout::contains(node.rect, local.x, local.y)))
        }
        walk(root, p, &self.motions)
    }

    /// Cursor shape to show at the current pointer position.
    pub fn cursor(&self) -> Cursor {
        if let Some(cursor) = self
            .components
            .edit_menu
            .cursor()
            .or_else(|| self.components.selects.cursor())
        {
            return cursor;
        }
        if let Some(drag) = self.components.splits.drag {
            return if drag.axis == Axis::Horizontal {
                Cursor::ResizeHorizontal
            } else {
                Cursor::ResizeVertical
            };
        }
        if !matches!(
            self.interaction.hovered,
            Some(Hit::Divider(..) | Hit::ScrollThumb(_) | Hit::ScrollTrack(_))
        ) && let Some(cursor) = self.css_cursor()
        {
            return cursor;
        }
        match self.interaction.hovered {
            Some(Hit::Button(_)) | Some(Hit::Select(_)) => Cursor::Hand,
            Some(Hit::Slider(id)) => {
                if self
                    .root
                    .as_ref()
                    .and_then(|r| r.find(id))
                    .and_then(|n| n.widget.as_ref())
                    .and_then(|w| w.slider())
                    .is_some_and(|s| s.read_only)
                {
                    Cursor::Arrow
                } else {
                    Cursor::Hand
                }
            }
            Some(Hit::Input(_)) => Cursor::Text,
            Some(Hit::Divider(_, Axis::Horizontal)) => Cursor::ResizeHorizontal,
            Some(Hit::Divider(_, Axis::Vertical)) => Cursor::ResizeVertical,
            Some(Hit::ScrollThumb(_)) | Some(Hit::ScrollTrack(_)) | Some(Hit::ScrollArea(_)) => {
                Cursor::Arrow
            }
            None => Cursor::Arrow,
        }
    }
}
