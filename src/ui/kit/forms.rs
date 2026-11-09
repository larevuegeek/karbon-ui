//! Forms: field, text input, list, check boxes, switch, radios, slider.
use super::{
    css,
    events::{OnChange, OnClick},
    icons::Icon,
    props::{Options, Text},
};
use crate::ui::{
    element::Element,
    template::{Component, Slots, TemplateContext},
};

/// Label, hint and error around any control.
#[derive(Default)]
pub struct Field {
    /// Label above the control.
    pub label: Text,
    /// Help under the control.
    pub hint: Text,
    /// Error under the control, in place of the hint.
    pub error: Text,
    /// Adds a required mark to the label.
    pub required: bool,
}
impl<R> Component<R> for Field {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        kit_ui!(cx, slots;
            <div class="k-field">
                {#if !self.label.0.is_empty()}
                    <div class="k-field-top">
                        <div class="k-label">{self.label.0}</div>
                        {#if self.required}<div class="k-required">*</div>{/if}
                    </div>
                {/if}
                <slot />
                {#if !self.error.0.is_empty()}
                    <div class="k-error">{self.error.0}</div>
                {:else if !self.hint.0.is_empty()}
                    <div class="k-hint">{self.hint.0}</div>
                {/if}
            </div>
        )
    }
}

/// One-line text field with its label, hint and error. Slot: `end`, inside on the right.
#[derive(Default)]
pub struct Input {
    /// Current value.
    pub value: Text,
    /// Sent at each change.
    pub oninput: OnChange<String>,
    /// Sent when Enter is pressed.
    pub onenter: OnClick,
    /// Text shown when empty.
    pub placeholder: Text,
    /// Label above the field.
    pub label: Text,
    /// Help under the field.
    pub hint: Text,
    /// Error under the field.
    pub error: Text,
    /// Icon inside the field, before the text.
    pub icon: &'static str,
    /// Border instead of a filled background.
    pub outline: bool,
    /// Hides the characters.
    pub password: bool,
    /// Search field.
    pub search: bool,
    /// Adds a required mark to the label.
    pub required: bool,
    /// Inactive.
    pub disabled: bool,
    /// Read-only.
    pub readonly: bool,
}
impl<R> Component<R> for Input {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let name = if self.label.0.is_empty() {
            self.placeholder.0.as_str()
        } else {
            self.label.0.as_str()
        };
        let input = self.oninput.emitter();
        let enter = self.onenter.action();
        let failed = !self.error.0.is_empty();
        kit_ui!(cx, slots;
            <Field label={self.label.0.as_str()} hint={self.hint.0.as_str()}
                error={self.error.0.as_str()} required={self.required}>
                <div class="k-input" class:k-input-outline={self.outline} class:k-input-error={failed}>
                    {#if !self.icon.is_empty()}
                        <Icon name={self.icon} />
                    {/if}
                    {#if self.password}
                        <input type="password" class="k-input-field" value={self.value.0.as_str()}
                            placeholder={self.placeholder.0.as_str()} aria-label={name}
                            oninput={input} onchange={enter} disabled={self.disabled} readonly={self.readonly}>
                    {:else if self.search}
                        <input type="search" class="k-input-field" value={self.value.0.as_str()}
                            placeholder={self.placeholder.0.as_str()} aria-label={name}
                            oninput={input} onchange={enter} disabled={self.disabled} readonly={self.readonly}>
                    {:else}
                        <input type="text" class="k-input-field" value={self.value.0.as_str()}
                            placeholder={self.placeholder.0.as_str()} aria-label={name}
                            oninput={input} onchange={enter} disabled={self.disabled} readonly={self.readonly}>
                    {/if}
                    <slot name="end" />
                </div>
            </Field>
        )
    }
}

/// Multi-line text field.
#[derive(Default)]
pub struct Textarea {
    /// Current value.
    pub value: Text,
    /// Sent at each change.
    pub oninput: OnChange<String>,
    /// Text shown when empty.
    pub placeholder: Text,
    /// Label above the field.
    pub label: Text,
    /// Help under the field.
    pub hint: Text,
    /// Error under the field.
    pub error: Text,
    /// Height in lines.
    pub rows: Option<f32>,
    /// Inactive.
    pub disabled: bool,
}
impl<R> Component<R> for Textarea {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        let name = if self.label.0.is_empty() {
            self.placeholder.0.as_str()
        } else {
            self.label.0.as_str()
        };
        let input = self.oninput.emitter();
        let rows = self.rows.unwrap_or(3.);
        kit_ui!(cx;
            <Field label={self.label.0.as_str()} hint={self.hint.0.as_str()} error={self.error.0.as_str()}>
                <div class="k-input k-input-multi">
                    <textarea class="k-input-field" value={self.value.0.as_str()} rows={rows}
                        placeholder={self.placeholder.0.as_str()} aria-label={name}
                        oninput={input} disabled={self.disabled}></textarea>
                </div>
            </Field>
        )
    }
}

/// Drop-down list; `options` is an array of labels or of `(value, label)` pairs.
#[derive(Default)]
pub struct Select {
    /// Value of the chosen option.
    pub value: Option<u64>,
    /// Choices.
    pub options: Options,
    /// Sent with the value of the chosen option.
    pub onchange: OnChange<u64>,
    /// Text shown without a choice.
    pub placeholder: Text,
    /// Label above the list.
    pub label: Text,
    /// Help under the list.
    pub hint: Text,
    /// Error under the list.
    pub error: Text,
    /// Inactive.
    pub disabled: bool,
}
impl<R> Component<R> for Select {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        let name = if self.label.0.is_empty() {
            self.placeholder.0.as_str()
        } else {
            self.label.0.as_str()
        };
        let change = self.onchange.emitter();
        let value = self.value;
        kit_ui!(cx;
            <Field label={self.label.0.as_str()} hint={self.hint.0.as_str()} error={self.error.0.as_str()}>
                <select class="k-select" value={value} placeholder={self.placeholder.0.as_str()}
                    aria-label={name} onchange={change} disabled={self.disabled}>
                    {#each self.options.0 as item (item.0)}
                        <option value={item.0}>{item.1}</option>
                    {/each}
                </select>
            </Field>
        )
    }
}

/// Check box with its label.
#[derive(Default)]
pub struct Checkbox {
    /// Checked state.
    pub checked: bool,
    /// Mixed state.
    pub indeterminate: bool,
    /// Text beside the box.
    pub label: Text,
    /// Sent with the new state.
    pub onchange: OnChange<bool>,
    /// Inactive.
    pub disabled: bool,
}
impl<R> Component<R> for Checkbox {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        let change = self.onchange.emitter();
        kit_ui!(cx;
            <div class="k-check">
                <input type="checkbox" checked={self.checked} indeterminate={self.indeterminate}
                    aria-label={self.label.0.as_str()} onchange={change} disabled={self.disabled}>
            </div>
        )
    }
}

/// On/off switch with its label.
#[derive(Default)]
pub struct Switch {
    /// On state.
    pub checked: bool,
    /// Text beside the switch.
    pub label: Text,
    /// Sent with the new state.
    pub onchange: OnChange<bool>,
    /// Inactive.
    pub disabled: bool,
}
impl<R> Component<R> for Switch {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        let change = self.onchange.emitter();
        kit_ui!(cx;
            <div class="k-check">
                <input type="checkbox" switch checked={self.checked}
                    aria-label={self.label.0.as_str()} onchange={change} disabled={self.disabled}>
            </div>
        )
    }
}

/// Group of exclusive choices.
#[derive(Default)]
pub struct RadioGroup {
    /// Value of the chosen option.
    pub value: Option<u64>,
    /// Choices.
    pub options: Options,
    /// Sent with the value of the chosen option.
    pub onchange: OnChange<u64>,
    /// Label above the group.
    pub label: Text,
    /// Lays the choices out on one row.
    pub row: bool,
    /// Inactive.
    pub disabled: bool,
}
impl<R> Component<R> for RadioGroup {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        let value = self.value;
        let change = self.onchange;
        kit_ui!(cx;
            <Field label={self.label.0.as_str()}>
                <div class="k-radios" class:k-radios-row={self.row}>
                    {#each self.options.0 as item (item.0)}
                        <div class="k-check">
                            <input type="radio" name="k-radio" checked={value == Some(item.0)}
                                aria-label={item.1.as_str()} onchange={change.action(item.0)} disabled={self.disabled}>
                        </div>
                    {/each}
                </div>
            </Field>
        )
    }
}

/// Value slider with its label.
pub struct Slider {
    /// Current value.
    pub value: f64,
    /// Lowest value.
    pub min: f64,
    /// Highest value.
    pub max: f64,
    /// Distance between values.
    pub step: f64,
    /// Sent while the thumb moves.
    pub oninput: OnChange<f64>,
    /// Sent when the thumb is released.
    pub onchange: OnChange<f64>,
    /// Label above the slider.
    pub label: Text,
    /// Inactive.
    pub disabled: bool,
}
impl Default for Slider {
    fn default() -> Self {
        Self {
            value: 0.,
            min: 0.,
            max: 100.,
            step: 1.,
            oninput: OnChange::default(),
            onchange: OnChange::default(),
            label: Text::default(),
            disabled: false,
        }
    }
}
impl<R> Component<R> for Slider {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        let name = self.label.0.clone();
        let input = self.oninput.emitter();
        let change = self.onchange.emitter();
        kit_ui!(cx;
            <Field label={self.label.0.as_str()}>
                <input type="range" class="k-slider" min={self.min} max={self.max} step={self.step}
                    value={self.value} aria-label={name.as_str()} oninput={input} onchange={change}
                    disabled={self.disabled}>
            </Field>
        )
    }
}
