//! Périphérique Direct3D 11 d'une surface partagée, sur la carte qu'on lui désigne.
//!
//! Il est à part de celui du renderer de la fenêtre : la surface vit sur la carte du
//! producteur (le moteur 3D), qui n'est pas toujours celle où Direct2D dessine
//! l'interface, et elle est présentée depuis le fil du producteur. DWM compose les deux
//! chaînes d'échange quelle que soit leur carte.
use super::super::graphics::Checked;
use crate::ui::shared_surface::AdapterLuid;
use windows::{
    Win32::{
        Foundation::{E_POINTER, HMODULE, LUID},
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_UNKNOWN},
            Direct3D11::{
                D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice,
                ID3D11Device, ID3D11DeviceContext,
            },
            Dxgi::{CreateDXGIFactory1, IDXGIAdapter, IDXGIDevice, IDXGIFactory4},
        },
    },
    core::Interface,
};

/// Le périphérique, son contexte immédiat, et la carte où ils vivent.
pub(super) struct Device {
    pub device: ID3D11Device,
    pub context: ID3D11DeviceContext,
    pub luid: AdapterLuid,
    pub name: String,
}

/// Périphérique matériel sur la carte `adapter` (`None` : la carte par défaut de
/// Direct3D, celle du renderer des fenêtres).
pub(super) fn create(adapter: Option<AdapterLuid>) -> crate::Result<Device> {
    let chosen = match adapter {
        Some(luid) => Some(adapter_of(luid)?),
        None => None,
    };
    let (mut device, mut context) = (None, None);
    // SAFETY : sorties optionnelles valides ; avec une carte explicite, Direct3D exige le
    // type de pilote « inconnu » ; aucun module logiciel.
    unsafe {
        D3D11CreateDevice(
            chosen.as_ref(),
            match chosen {
                Some(_) => D3D_DRIVER_TYPE_UNKNOWN,
                None => D3D_DRIVER_TYPE_HARDWARE,
            },
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )
    }
    .check("D3D11CreateDevice")?;
    let missing = || crate::Error::Platform {
        operation: "D3D11CreateDevice",
        code: E_POINTER.0,
    };
    let device = device.ok_or_else(missing)?;
    let context = context.ok_or_else(missing)?;
    let dxgi: IDXGIDevice = device.cast().check("QueryInterface IDXGIDevice")?;
    // SAFETY : périphérique DXGI valide ; windows-rs gère la référence renvoyée.
    let adapter = unsafe { dxgi.GetAdapter() }.check("IDXGIDevice::GetAdapter")?;
    // SAFETY : carte valide ; description copiée par valeur.
    let description = unsafe { adapter.GetDesc() }.check("IDXGIAdapter::GetDesc")?;
    let name = String::from_utf16_lossy(&description.Description);
    Ok(Device {
        device,
        context,
        luid: bytes(description.AdapterLuid),
        name: name.trim_end_matches('\0').to_string(),
    })
}

/// La carte de numéro `luid`.
fn adapter_of(luid: AdapterLuid) -> crate::Result<IDXGIAdapter> {
    // SAFETY : appel sans pointeur ; l'IID provient du type demandé.
    let factory: IDXGIFactory4 = unsafe { CreateDXGIFactory1() }.check("CreateDXGIFactory1")?;
    // SAFETY : fabrique valide ; le numéro est passé par valeur.
    unsafe { factory.EnumAdapterByLuid(structure(luid)) }.check("EnumAdapterByLuid")
}

/// Les huit octets d'un LUID, dans l'ordre de la mémoire (celui que Vulkan recopie).
fn bytes(luid: LUID) -> AdapterLuid {
    let mut bytes = [0; 8];
    bytes[..4].copy_from_slice(&luid.LowPart.to_le_bytes());
    bytes[4..].copy_from_slice(&luid.HighPart.to_le_bytes());
    bytes
}

fn structure(luid: AdapterLuid) -> LUID {
    let [a, b, c, d, e, f, g, h] = luid;
    LUID {
        LowPart: u32::from_le_bytes([a, b, c, d]),
        HighPart: i32::from_le_bytes([e, f, g, h]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_luid_keeps_its_bytes_both_ways() {
        let luid = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];
        assert_eq!(bytes(structure(luid)), luid);
        assert_eq!(structure(luid).LowPart, 0x7856_3412);
    }
}
