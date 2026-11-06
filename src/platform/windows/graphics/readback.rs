//! Rendu hors écran : texture D3D11 copiée dans une texture de relecture, puis dans la
//! mémoire de l'appelant.
use super::*;

impl Renderer {
    pub(super) fn readback_target(
        &self,
        width: u32,
        height: u32,
        scale: f32,
    ) -> crate::Result<Target> {
        let mut desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let mut texture = None;
        // SAFETY: description valide, sans données initiales ; sortie optionnelle valide.
        let gpu = self.gpu.gpu();
        unsafe { gpu.device.CreateTexture2D(&desc, None, Some(&mut texture)) }
            .check("CreateTexture2D")?;
        let texture = required(texture)?;
        desc.Usage = D3D11_USAGE_STAGING;
        desc.BindFlags = 0;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        let mut staging = None;
        // SAFETY: même description, convertie en texture de relecture CPU.
        unsafe { gpu.device.CreateTexture2D(&desc, None, Some(&mut staging)) }
            .check("CreateTexture2D staging")?;
        let staging = required(staging)?;
        let surface: IDXGISurface = texture.cast().check("QueryInterface")?;
        let target = device::target(&gpu.d2d, &surface, scale)?;
        Ok(Target {
            output: Output::Readback { texture, staging },
            width,
            height,
            canvas: Canvas::new(target)?,
        })
    }

    pub(in crate::platform::windows) fn render(
        &mut self,
        scene: &Scene,
        width: u32,
        height: u32,
        scale: f32,
    ) -> crate::Result<Vec<u8>> {
        self.draw_scene(scene, width, height, scale, Kind::Readback)?;
        let mut pixels = vec![0; width as usize * height as usize * 4];
        self.read_pixels_into(&mut pixels)?;
        Ok(pixels)
    }

    /// Relire directement dans une surface persistante fournie par l'appelant.
    pub(in crate::platform::windows) fn render_into(
        &mut self,
        scene: &Scene,
        width: u32,
        height: u32,
        scale: f32,
        pixels: &mut [u8],
    ) -> crate::Result<()> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4));
        if expected != Some(pixels.len()) {
            return Err(crate::Error::invalid_input(
                "destination buffer size differs from the render size",
            ));
        }
        self.draw_scene(scene, width, height, scale, Kind::Readback)?;
        self.read_pixels_into(pixels)
    }

    fn read_pixels_into(&self, pixels: &mut [u8]) -> crate::Result<()> {
        let t = self.target.as_ref().unwrap();
        let Output::Readback { texture, staging } = &t.output else {
            return Err(crate::Error::invalid_input(
                "the presentation target cannot be read back",
            ));
        };
        copy_to_cpu(
            &self.gpu.gpu().context,
            texture,
            staging,
            t.width,
            t.height,
            pixels,
        )
    }
}

/// Copie une texture GPU dans `staging`, la mappe puis recopie ses lignes dans `pixels`.
pub(super) fn copy_to_cpu(
    context: &ID3D11DeviceContext,
    texture: &ID3D11Texture2D,
    staging: &ID3D11Texture2D,
    width: u32,
    height: u32,
    pixels: &mut [u8],
) -> crate::Result<()> {
    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    // SAFETY: textures de même description, créées par ce périphérique ; sortie valide.
    unsafe {
        context.CopyResource(staging, texture);
        context.Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
    }
    .check("D3D11 Map")?;
    struct Mapping<'a>(&'a ID3D11DeviceContext, &'a ID3D11Texture2D);
    impl Drop for Mapping<'_> {
        fn drop(&mut self) {
            // SAFETY: libère exactement le Map réussi ci-dessus.
            unsafe { self.0.Unmap(self.1, 0) };
        }
    }
    let _mapping = Mapping(context, staging);
    let row = width as usize * 4;
    if mapped.pData.is_null() || (mapped.RowPitch as usize) < row {
        return Err(crate::Error::invalid_input("invalid mapped texture"));
    }
    debug_assert_eq!(pixels.len(), row * height as usize);
    for y in 0..height as usize {
        // SAFETY: la taille et le row pitch proviennent de la texture mappée ci-dessus,
        // qui reste mappée jusqu'à la fin de la fonction.
        unsafe {
            std::ptr::copy_nonoverlapping(
                mapped.pData.cast::<u8>().add(y * mapped.RowPitch as usize),
                pixels.as_mut_ptr().add(y * row),
                row,
            )
        };
    }
    Ok(())
}
