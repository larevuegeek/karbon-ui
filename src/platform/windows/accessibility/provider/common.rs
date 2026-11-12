//! Méthodes communes des fournisseurs : propriétés, patterns, navigation, valeur.
use super::*;

impl Core {
    pub(in super::super) fn node(&self) -> Result<Arc<Node>> {
        self.bridge.node(self.key)
    }
    pub(super) fn options(&self) -> Result<ProviderOptions> {
        self.node()?;
        // ServerSideProvider | RefuseNonClientSupport.
        Ok(ProviderOptions(2 | 0x40))
    }
    pub(super) fn pattern(&self, pattern: i32) -> Result<IUnknown> {
        let node = self.node()?;
        let (bridge, key) = (&self.bridge, self.key);
        Ok(match pattern {
            10000 if node.invokable => bridge.interface::<IInvokeProvider>(key)?.into(),
            10002 if node.value.is_some() => bridge.interface::<IValueProvider>(key)?.into(),
            10014 if node.text.is_some() => bridge.interface::<ITextProvider>(key)?.into(),
            10003 if node.range.is_some() => bridge.interface::<IRangeValueProvider>(key)?.into(),
            10015 if matches!(node.choice, Some(ChoiceState::Toggle(_))) => {
                bridge.interface::<IToggleProvider>(key)?.into()
            }
            10001 if node.selection_container() => {
                bridge.interface::<ISelectionProvider>(key)?.into()
            }
            10005 if node.expanded.is_some() => {
                bridge.interface::<IExpandCollapseProvider>(key)?.into()
            }
            10010 if matches!(node.choice, Some(ChoiceState::Radio(_))) => {
                bridge.interface::<ISelectionItemProvider>(key)?.into()
            }
            _ => return nothing(),
        })
    }
    pub(super) fn property(&self, property: i32) -> Result<Variant> {
        let n = self.node()?;
        Ok(match property {
            30001 => Variant::rectangle(n.bounds)?,
            // SAFETY: lecture de l'identifiant du processus courant, sans argument.
            30002 => Variant::integer(unsafe { GetCurrentProcessId() } as i32),
            30003 => Variant::integer(n.control_type),
            30005 => Variant::string(&n.name)?,
            30008 => Variant::boolean(n.focused),
            30009 => Variant::boolean(n.focusable),
            30010 => Variant::boolean(n.enabled),
            30011 => Variant::string(&n.key.automation_id())?,
            30012 => Variant::string(if n.is_dialog {
                "Dialog"
            } else if n.control_type == 50022 {
                "Tooltip"
            } else if n.control_type == 50009 {
                "Menu"
            } else if n.control_type == 50011 {
                "MenuItem"
            } else if n.expanded.is_some() {
                "Select"
            } else if n.control_type == 50007 {
                "SelectOption"
            } else if n.range.is_some() {
                "Slider"
            } else if n.control_type == 50019 {
                "Tab"
            } else if n.control_type == 50018 {
                "TabList"
            } else if n.control_type == 50026 {
                "RadioGroup"
            } else if matches!(n.choice, Some(ChoiceState::Radio(_))) {
                "RadioButton"
            } else if matches!(n.choice, Some(ChoiceState::Toggle(_))) {
                if n.control_type == 50002 {
                    "Checkbox"
                } else {
                    "Switch"
                }
            } else if n.value.is_some() {
                "TextInput"
            } else if n.invokable {
                "Button"
            } else {
                "Element"
            })?,
            30013 => Variant::string(&n.help)?,
            30016 => Variant::boolean(true),
            30017 => Variant::boolean(n.control_type != 50022),
            30019 => Variant::boolean(false),
            30022 => Variant::boolean(n.offscreen),
            30024 => Variant::string("karbon-ui")?,
            30174 => Variant::boolean(n.is_dialog),
            30031 => Variant::boolean(n.invokable),
            30043 => Variant::boolean(n.value.is_some()),
            30033 => Variant::boolean(n.range.is_some()),
            30028 => Variant::boolean(n.expanded.is_some()),
            30070 if n.expanded.is_some() => Variant::integer(i32::from(n.expanded.unwrap())),
            30023 | 30047..=30052 if n.range.is_some() => {
                range_value::property(n.range.unwrap(), property).variant()
            }
            30040 => Variant::boolean(n.text.is_some()),
            30045 if n.value.is_some() => Variant::string(n.value.as_deref().unwrap())?,
            30046 if n.value.is_some() => Variant::boolean(n.read_only),
            30107 => Variant::string("Karbon UI native UI Automation provider")?,
            // AriaProperties : `aria-current`, comme les navigateurs.
            30102 if n.current.is_some() => {
                Variant::string(&format!("current={}", n.current.unwrap()))?
            }
            _ => choices::property(&self.bridge, &n, property)?,
        })
    }
    pub(super) fn host(&self) -> Result<IRawElementProviderSimple> {
        self.node()?;
        if self.key != NodeId::Root {
            return nothing();
        }
        // SAFETY: handle de la fenêtre propriétaire du pont, transmis sans déréférencement.
        unsafe { UiaHostProviderFromHwnd(HWND(self.bridge.window as Hwnd)) }
    }
    pub(super) fn navigate(&self, direction: i32) -> Result<IRawElementProviderFragment> {
        let target = lock(&self.bridge.snapshot).navigate(self.key, direction)?;
        match target {
            Some(key) => self.bridge.interface(key),
            None => nothing(),
        }
    }
    pub(super) fn runtime_id(&self) -> Result<*mut SAFEARRAY> {
        self.node()?;
        let (kind, value, option) = match self.key {
            NodeId::Root => return Ok(null_mut()),
            NodeId::Element(id) => (2, id, None),
            NodeId::Caption(id) => (1, id as u64, None),
            NodeId::SelectPopup(id) => (3, id, None),
            NodeId::SelectOption(owner, key) => (4, owner, Some(key)),
            NodeId::Tooltip(owner) => (5, owner, None),
        };
        let mut values = vec![3, kind, value as u32 as i32, (value >> 32) as u32 as i32];
        if let Some(option) = option {
            values.extend([option as u32 as i32, (option >> 32) as u32 as i32]);
        }
        integers(&values)
    }
    pub(super) fn element_from_point(&self, x: f64, y: f64) -> Result<IRawElementProviderFragment> {
        self.node()?;
        let key = {
            let snapshot = lock(&self.bridge.snapshot);
            snapshot
                .nodes
                .iter()
                .rev()
                .find(|n| !n.offscreen && contains(&n.clipped, x, y))
                .map(|n| n.key)
        };
        match key {
            Some(key) => self.bridge.interface(key),
            None => nothing(),
        }
    }
    pub(super) fn focused_element(&self) -> Result<IRawElementProviderFragment> {
        self.node()?;
        let key = lock(&self.bridge.snapshot)
            .nodes
            .iter()
            .find(|n| n.focused)
            .map(|n| n.key);
        match key {
            Some(key) => self.bridge.interface(key),
            None => nothing(),
        }
    }
    pub(super) fn set_value(&self, value: PCWSTR) -> Result<()> {
        if value.is_null() {
            return Err(fail(POINTER));
        }
        let mut len = 0;
        // SAFETY: chaîne terminée par zéro fournie par l'appelant ; lecture bornée à 256 Kio.
        while len <= 262_144 && unsafe { *value.0.add(len) } != 0 {
            len += 1;
        }
        if len > 262_144 {
            return Err(fail(INVALID_ARGUMENT));
        }
        // SAFETY: les `len` unités précédant le zéro terminal viennent d'être lues.
        let text = String::from_utf16(unsafe { std::slice::from_raw_parts(value.0, len) })
            .map_err(|_| fail(INVALID_ARGUMENT))?;
        check(self.bridge.dispatch(Command::SetValue(self.key, text)))
    }
    pub(super) fn value(&self) -> Result<BSTR> {
        let n = self.node()?;
        bstr(n.value.as_deref().ok_or_else(|| fail(UNSUPPORTED))?)
    }
    pub(super) fn value_read_only(&self) -> Result<BOOL> {
        let n = self.node()?;
        if n.value.is_none() {
            return Err(fail(UNSUPPORTED));
        }
        Ok(n.read_only.into())
    }
    pub(super) fn invoke(&self) -> Result<()> {
        check(self.bridge.dispatch(Command::Invoke(self.key)))
    }
}
