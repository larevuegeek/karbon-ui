use super::*;
impl Tooltips {
    pub fn reset(&mut self) -> bool {
        let was = self.visible;
        *self = Self::default();
        was
    }

    pub fn next_wake(&self) -> Option<Instant> {
        self.due
    }

    pub fn snapshot(&self) -> Option<TooltipInfo> {
        let t = self.target.as_ref()?;
        let bounds = self.bounds.filter(|_| self.visible)?;
        (layout::width(bounds) > 0. && layout::height(bounds) > 0.).then(|| TooltipInfo {
            owner: t.id,
            text: Arc::clone(&t.tooltip.text),
            bounds,
        })
    }

    pub fn dismiss(&mut self) -> bool {
        self.suppressed = self.target.as_ref().map(|t| t.id);
        self.due = None;
        let was = self.visible;
        self.visible = false;
        self.bounds = None;
        was
    }

    pub fn tick(&mut self, now: Instant) -> bool {
        if self.due.is_some_and(|due| now >= due) {
            self.due = None;
            self.visible = true;
            return true;
        }
        false
    }

    pub fn sync(
        &mut self,
        root: &Element,
        pointer: Option<PointF>,
        focus: Option<ElementId>,
        viewport: RectF,
        blocked: bool,
        now: Instant,
    ) -> bool {
        if blocked {
            let was = self.visible;
            self.visible = false;
            self.bounds = None;
            self.due = None;
            self.target = None;
            return was;
        }
        fn hover(node: &Element, p: PointF) -> Option<&Element> {
            if !layout::contains(node.clip, p.x, p.y) {
                return None;
            }
            // Le clip d'un élément sans overflow est celui de son parent : seul son
            // propre rectangle visible porte son infobulle.
            node.children
                .iter()
                .rev()
                .find_map(|n| hover(n, p))
                .or_else(|| {
                    node.tooltip
                        .as_ref()
                        .filter(|_| {
                            layout::contains(layout::intersect(node.rect, node.clip), p.x, p.y)
                        })
                        .map(|_| node)
                })
        }
        let in_bubble = self.snapshot().is_some_and(|t| {
            pointer.is_some_and(|p| {
                let a = self.target.as_ref().unwrap().anchor;
                let r = t.bounds;
                let corridor = RectF {
                    left: a.left.max(r.left),
                    right: a.right.min(r.right),
                    top: a.bottom.min(r.bottom),
                    bottom: a.top.max(r.top),
                };
                layout::contains(r, p.x, p.y) || layout::contains(corridor, p.x, p.y)
            })
        });
        let candidate = if in_bubble {
            self.target.as_ref().and_then(|t| root.find(t.id))
        } else {
            pointer.and_then(|p| hover(root, p)).or_else(|| {
                focus
                    .and_then(|id| root.find(id))
                    .filter(|n| n.tooltip.is_some())
            })
        };
        let candidate = candidate.filter(|n| {
            n.tooltip.is_some() && layout::width(n.clip) > 0. && layout::height(n.clip) > 0.
        });
        let next = candidate.map(|n| Target {
            id: n.id,
            tooltip: (**n.tooltip.as_ref().unwrap()).clone(),
            anchor: layout::intersect(n.rect, n.clip),
            viewport,
        });
        if next.as_ref().map(|n| n.id) != self.suppressed {
            self.suppressed = None;
        }
        let unchanged = match (&self.target, &next) {
            (Some(a), Some(b)) => {
                a.id == b.id
                    && a.tooltip == b.tooltip
                    && same(a.anchor, b.anchor)
                    && same(a.viewport, b.viewport)
            }
            (None, None) => true,
            _ => false,
        };
        if unchanged {
            return false;
        }
        let was = self.visible;
        self.target = next;
        self.visible = false;
        self.bounds = None;
        self.due = self
            .target
            .as_ref()
            .filter(|t| Some(t.id) != self.suppressed)
            .map(|t| now + t.tooltip.delay);
        let appeared = self.tick(now);
        was || appeared
    }

    pub fn measure(&mut self, text: &mut dyn TextMeasurer) -> crate::Result<bool> {
        if !self.visible || self.bounds.is_some() {
            return Ok(false);
        }
        let Some(t) = &self.target else {
            return Ok(false);
        };
        let width = t.tooltip.max_width.min(layout::width(t.viewport));
        let available = (width - 20.).max(1.);
        let measured =
            text.measure_text(&t.tooltip.text, t.tooltip.style.font_size, false, available)?;
        let size = SizeF {
            width: (measured.width.ceil() + 20.).min(width),
            height: measured.height.ceil() + 16.,
        };
        self.bounds = Some(AnchoredLayer::place(t.anchor, size, t.viewport).bounds);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un élément sans overflow hérite du clip de son parent : un voisin plus loin dans
    /// l'arbre ne doit pas voler l'infobulle de l'élément réellement survolé.
    #[test]
    fn hover_picks_the_element_under_the_pointer_not_a_sibling_sharing_its_clip() {
        let page = RectF::new(0., 0., 200., 100.);
        let mut first = Element::column(2, Vec::new()).tooltip("Premier");
        first.rect = RectF::new(0., 0., 100., 20.);
        first.clip = page;
        let mut last = Element::column(3, Vec::new()).tooltip("Dernier");
        last.rect = RectF::new(0., 60., 100., 80.);
        last.clip = page;
        let mut root = Element::column(1, vec![first, last]);
        root.rect = page;
        root.clip = page;
        let mut tooltips = Tooltips::default();
        let now = Instant::now();
        let at = |x, y| Some(PointF { x, y });
        tooltips.sync(&root, at(10., 10.), None, page, false, now);
        assert_eq!(tooltips.target.as_ref().map(|t| t.id), Some(ElementId(2)));
        tooltips.sync(&root, at(10., 70.), None, page, false, now);
        assert_eq!(tooltips.target.as_ref().map(|t| t.id), Some(ElementId(3)));
        // Entre les deux : aucune infobulle.
        tooltips.sync(&root, at(10., 40.), None, page, false, now);
        assert!(tooltips.target.is_none());
    }
}
