//! Tampons en mémoire partagée (`wl_shm`) : une image ARGB8888 prémultipliée par tampon
//! (BGRA en mémoire, le format du moteur), réutilisée dès que le compositeur l'a rendue.
//! Chaque tampon garde les zones redessinées depuis qu'il a été rempli : seules elles
//! sont recopiées depuis l'image de la fenêtre.
use super::{
    display::{Kind, NEW, Ptr, create, destroy},
    protocol::{wl_buffer, wl_shm, wl_shm_pool},
};
use crate::platform::linux::ffi::wayland::{wl_buffer_interface, wl_shm_pool_interface};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

/// Format `WL_SHM_FORMAT_ARGB8888` (toujours pris en charge par un compositeur).
const ARGB8888: u32 = 0;

pub(super) struct Buffer {
    pub raw: Ptr,
    map: *mut u8,
    len: usize,
    pub width: usize,
    pub height: usize,
    /// Attaché à la surface et pas encore rendu par le compositeur.
    pub busy: bool,
    /// Zones en retard sur l'image de la fenêtre (`None` : tout le tampon).
    stale: Option<Vec<[i32; 4]>>,
}
impl Buffer {
    pub fn new(shm: Ptr, width: usize, height: usize) -> crate::Result<Self> {
        let len = width * height * 4;
        let size =
            i32::try_from(len).map_err(|_| crate::Error::invalid_input("buffer too large"))?;
        // SAFETY : nom statique ; descripteur possédé ci-dessous.
        let fd = unsafe { libc::memfd_create(c"karbon-buffer".as_ptr(), libc::MFD_CLOEXEC) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        // SAFETY : descripteur fraîchement créé.
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        // SAFETY : descripteur valide ; taille positive.
        if unsafe { libc::ftruncate(fd.as_raw_fd(), len as libc::off_t) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        // SAFETY : projection partagée de tout le fichier, libérée par `Drop`.
        let map = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd.as_raw_fd(),
                0,
            )
        };
        if map == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error().into());
        }
        // libwayland duplique le descripteur à l'envoi : il peut être fermé ensuite.
        let pool = create!(
            shm,
            wl_shm::CREATE_POOL,
            &raw const wl_shm_pool_interface,
            Kind::Silent,
            NEW,
            fd.as_raw_fd(),
            size
        );
        let raw = create!(
            pool,
            wl_shm_pool::CREATE_BUFFER,
            &raw const wl_buffer_interface,
            Kind::Buffer,
            NEW,
            0i32,
            width as i32,
            height as i32,
            (width * 4) as i32,
            ARGB8888
        );
        destroy!(pool, wl_shm_pool::DESTROY);
        Ok(Self {
            raw,
            map: map.cast(),
            len,
            width,
            height,
            busy: false,
            stale: None,
        })
    }
    /// L'image de la fenêtre a changé dans `zones` (`None` : partout).
    pub fn invalidate(&mut self, zones: Option<&[[i32; 4]]>) {
        match (&mut self.stale, zones) {
            (Some(stale), Some(zones)) if stale.len() + zones.len() <= 64 => {
                stale.extend_from_slice(zones);
            }
            _ => self.stale = None,
        }
    }
    /// Recopie depuis `image` (même taille, `width × 4` octets par ligne) les zones en
    /// retard.
    pub fn fill(&mut self, image: &[u8]) {
        let row = self.width * 4;
        let (w, h) = (self.width as i32, self.height as i32);
        let stale = self.stale.replace(Vec::new());
        let pixels = self.pixels();
        let Some(zones) = stale else {
            pixels.copy_from_slice(&image[..pixels.len()]);
            return;
        };
        for [left, top, right, bottom] in zones {
            let (left, right) = (left.clamp(0, w) as usize, right.clamp(0, w) as usize);
            for y in top.clamp(0, h) as usize..bottom.clamp(0, h) as usize {
                let span = y * row + left * 4..y * row + right * 4;
                pixels[span.clone()].copy_from_slice(&image[span]);
            }
        }
    }
    /// Pixels du tampon (`width × 4` octets par ligne).
    fn pixels(&mut self) -> &mut [u8] {
        // SAFETY : projection de `len` octets, exclusive tant que le tampon n'est pas
        // attaché (le compositeur ne lit qu'un tampon occupé).
        unsafe { std::slice::from_raw_parts_mut(self.map, self.len) }
    }
}
impl Drop for Buffer {
    fn drop(&mut self) {
        destroy!(self.raw, wl_buffer::DESTROY);
        // SAFETY : projection possédée, libérée une fois.
        unsafe { libc::munmap(self.map.cast(), self.len) };
    }
}
