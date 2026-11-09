//! Pagination.
use super::{css, events::OnChange, icons::Icon};
use crate::ui::{
    element::Element,
    template::{Component, Slots, TemplateContext},
};

/// Page numbers; pages count from 1.
#[derive(Default)]
pub struct Pagination {
    /// Current page.
    pub page: usize,
    /// Number of pages.
    pub pages: usize,
    /// Sent with the chosen page.
    pub onchange: OnChange<usize>,
}
/// Numéros à montrer ; 0 marque un saut.
fn page_numbers(page: usize, pages: usize) -> Vec<usize> {
    let mut out = Vec::new();
    for n in 1..=pages {
        if n == 1 || n == pages || n.abs_diff(page) <= 1 {
            if out.last().is_some_and(|last| n - last > 1) {
                out.push(0);
            }
            out.push(n);
        }
    }
    out
}
impl<R> Component<R> for Pagination {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        let (page, pages) = (self.page.max(1), self.pages.max(1));
        let numbers: Vec<(usize, usize)> =
            page_numbers(page, pages).into_iter().enumerate().collect();
        let change = self.onchange;
        kit_ui!(cx;
            <nav class="k-pages" aria-label="Pagination">
                <button class="k-page-btn" aria-label="Previous" disabled={page <= 1}
                    onclick={change.action(page.saturating_sub(1).max(1))}><Icon name="chevron-left" /></button>
                {#each numbers as item (item.0)}
                    {#if item.1 == 0}
                        <div class="k-muted k-page-gap">&hellip;</div>
                    {:else}
                        <button class="k-page-btn" class:k-page-active={item.1 == page}
                            onclick={change.action(item.1)}><div>{item.1}</div></button>
                    {/if}
                {/each}
                <button class="k-page-btn" aria-label="Next" disabled={page >= pages}
                    onclick={change.action((page + 1).min(pages))}><Icon name="chevron-right" /></button>
            </nav>
        )
    }
}
