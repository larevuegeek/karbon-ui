//! Texture Direct3D 11 partagée avec un autre renderer : poignée NT, verrou à clé, et sa
//! relecture pour les contrôles.
use super::super::graphics::Checked;
use windows::{
    Win32::{
        Foundation::{CloseHandle, E_POINTER, HANDLE, S_OK},
        Graphics::{
            Direct3D11::{
                D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_READ,
                D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX,
                D3D11_RESOURCE_MISC_SHARED_NTHANDLE, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
                D3D11_USAGE_STAGING, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
            },
            Dxgi::{
                Common::{DXGI_FORMAT_B8G8R8A8_UNORM_SRGB, DXGI_SAMPLE_DESC},
                DXGI_SHARED_RESOURCE_READ, DXGI_SHARED_RESOURCE_WRITE, IDXGIKeyedMutex,
                IDXGIResource1,
            },
        },
    },
    core::{Interface, PCWSTR},
};

/// Clé du verrou quand la texture est libre pour le producteur (son état à la création).
const FREE: u64 = 0;
/// Clé du verrou quand une image dessinée attend d'être montrée.
const DRAWN: u64 = 1;
/// Attente au plus de l'image du producteur, en millisecondes.
const ACQUIRE_TIMEOUT_MS: u32 = 1000;

/// La texture où le producteur dessine.
pub(super) struct Texture {
    pub texture: ID3D11Texture2D,
    mutex: IDXGIKeyedMutex,
    handle: HANDLE,
    /// Copie relisible par le processeur, faite à la première relecture.
    staging: Option<ID3D11Texture2D>,
    pub width: u32,
    pub height: u32,
}

impl Texture {
    pub(super) fn new(device: &ID3D11Device, width: u32, height: u32) -> crate::Result<Self> {
        if width == 0 || height == 0 || width > 16384 || height > 16384 {
            return Err(crate::Error::invalid_input("invalid shared surface size"));
        }
        let shared =
            D3D11_RESOURCE_MISC_SHARED_NTHANDLE.0 | D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX.0;
        let texture = create(device, &description(width, height, false, shared as u32))?;
        let mutex: IDXGIKeyedMutex = texture.cast().check("QueryInterface IDXGIKeyedMutex")?;
        let resource: IDXGIResource1 = texture.cast().check("QueryInterface IDXGIResource1")?;
        let access = DXGI_SHARED_RESOURCE_READ.0 | DXGI_SHARED_RESOURCE_WRITE.0;
        // SAFETY : ressource partageable par poignée NT (drapeau ci-dessus) ; ni attributs
        // de sécurité ni nom ; la poignée rendue appartient à `Texture` (`CloseHandle`).
        let handle = unsafe { resource.CreateSharedHandle(None, access, PCWSTR::null()) }
            .check("CreateSharedHandle")?;
        Ok(Self {
            texture,
            mutex,
            handle,
            staging: None,
            width,
            height,
        })
    }

    /// La poignée NT de la texture, pour le producteur.
    pub(super) fn handle(&self) -> isize {
        self.handle.0 as isize
    }

    /// Prend la texture quand le producteur l'a rendue dessinée.
    pub(super) fn acquire_drawn(&self) -> crate::Result<()> {
        // `AcquireSync` rend `WAIT_TIMEOUT` et `WAIT_ABANDONED` comme des succès de
        // HRESULT : le code brut est lu, seul `S_OK` donne la texture.
        // SAFETY : verrou de cette texture, vivant ; appel par valeur, attente bornée.
        let code = unsafe {
            (Interface::vtable(&self.mutex).AcquireSync)(
                Interface::as_raw(&self.mutex),
                DRAWN,
                ACQUIRE_TIMEOUT_MS,
            )
        };
        if code == S_OK {
            Ok(())
        } else if code.is_ok() {
            Err(crate::Error::TimedOut {
                operation: "IDXGIKeyedMutex::AcquireSync",
            })
        } else {
            Err(windows::core::Error::from(code)).check("IDXGIKeyedMutex::AcquireSync")
        }
    }

    /// Rend la texture au producteur, libre pour l'image suivante.
    pub(super) fn release_free(&self) -> crate::Result<()> {
        // SAFETY : verrou pris par `acquire_drawn` sur ce fil.
        unsafe { self.mutex.ReleaseSync(FREE) }.check("IDXGIKeyedMutex::ReleaseSync")
    }

    /// Les pixels BGRA de la texture, lignes de haut en bas (verrou pris par l'appelant).
    pub(super) fn read(
        &mut self,
        device: &ID3D11Device,
        context: &ID3D11DeviceContext,
    ) -> crate::Result<Vec<u8>> {
        if self.staging.is_none() {
            let readable = description(self.width, self.height, true, 0);
            self.staging = Some(create(device, &readable)?);
        }
        let staging = self.staging.as_ref().expect("made above");
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        // SAFETY : textures de même taille et de même format, de ce périphérique ; sortie
        // locale ; `Map` attend la fin de la copie.
        unsafe {
            context.CopyResource(staging, &self.texture);
            context.Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
        }
        .check("D3D11 Map")?;
        let row = self.width as usize * 4;
        let mut pixels = vec![0; row * self.height as usize];
        let valid = !mapped.pData.is_null() && mapped.RowPitch as usize >= row;
        if valid {
            for (y, line) in pixels.chunks_exact_mut(row).enumerate() {
                // SAFETY : la texture reste projetée jusqu'à `Unmap` ; chaque ligne a
                // `RowPitch` octets, dont `row` sont lus, pour `height` lignes.
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        mapped.pData.cast::<u8>().add(y * mapped.RowPitch as usize),
                        line.as_mut_ptr(),
                        row,
                    )
                };
            }
        }
        // SAFETY : libère exactement le `Map` réussi ci-dessus.
        unsafe { context.Unmap(staging, 0) };
        if !valid {
            return Err(crate::Error::invalid_input("invalid mapped texture"));
        }
        Ok(pixels)
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        // SAFETY : poignée rendue par `CreateSharedHandle`, fermée une fois ; la texture
        // vit tant qu'un des deux côtés la tient.
        let _ = unsafe { CloseHandle(self.handle) };
    }
}

/// Description d'une texture BGRA sRGB de cette taille : cible du producteur avec les
/// drapeaux `misc`, ou (`readable`) copie relisible par le processeur.
fn description(width: u32, height: u32, readable: bool, misc: u32) -> D3D11_TEXTURE2D_DESC {
    D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        // Le format des fenêtres d'un moteur (Vulkan : `VK_FORMAT_B8G8R8A8_SRGB`) ; la
        // chaîne d'échange, sans le suffixe sRGB, en reçoit les octets tels quels.
        Format: DXGI_FORMAT_B8G8R8A8_UNORM_SRGB,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: if readable {
            D3D11_USAGE_STAGING
        } else {
            D3D11_USAGE_DEFAULT
        },
        BindFlags: if readable {
            0
        } else {
            (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32
        },
        CPUAccessFlags: if readable {
            D3D11_CPU_ACCESS_READ.0 as u32
        } else {
            0
        },
        MiscFlags: misc,
    }
}

fn create(device: &ID3D11Device, desc: &D3D11_TEXTURE2D_DESC) -> crate::Result<ID3D11Texture2D> {
    let mut texture = None;
    // SAFETY : description locale valide, sans données initiales ; sortie optionnelle.
    unsafe { device.CreateTexture2D(desc, None, Some(&mut texture)) }.check("CreateTexture2D")?;
    texture.ok_or(crate::Error::Platform {
        operation: "CreateTexture2D",
        code: E_POINTER.0,
    })
}
