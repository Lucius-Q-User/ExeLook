use std::{
    io,
    ffi::CStr,
    str::Utf8Error,
    option::NoneError,
    convert::{From, TryInto}
};

use pelite::{
    self,
    PeFile,
    FileMap,
    resources::{Resources, FindError}
};

use crate::{
    dib::{
        self,
        BitmapInfoHeader
    }
};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Pe(pelite::Error),
    UTF(Utf8Error),
    NoIconFound,
    PlanarNotSupported,
    UnrecognizedBPP,
    UnknownCompression,
    MalformedPng
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Error::UTF(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<pelite::Error> for Error {
    fn from(err: pelite::Error) -> Self {
        Error::Pe(err)
    }
}

impl From<NoneError> for Error {
    fn from(_err: NoneError) -> Self {
        Error::NoIconFound
    }
}

impl From<FindError> for Error {
    fn from(_err: FindError) -> Self {
        Error::NoIconFound
    }
}

struct PngHeader<'a> {
    bytes: &'a [u8]
}

impl<'a> PngHeader<'a> {
    fn from_bytes<'b>(bytes: &'b [u8]) -> Result<PngHeader<'b>> {
        if bytes.len() < 24 || bytes[12..16] != [b'I', b'H', b'D', b'R'] {
            Err(Error::MalformedPng)
        } else {
            Ok(PngHeader {bytes})
        }
    }
    fn width(&self) -> i32 {
        u32::from_be_bytes(self.bytes[16..20].try_into().unwrap()) as i32
    }
    fn height(&self) -> i32 {
        u32::from_be_bytes(self.bytes[20..24].try_into().unwrap()) as i32
    }
}

pub type Result<T> = ::std::result::Result<T, Error>;

fn get_resources(bytes: &[u8]) -> Result<Resources> {
    let res = PeFile::from_bytes(bytes)?.resources();
    if let Err(pelite::Error::Null) = res {
        Err(Error::NoIconFound)
    } else {
        res.map_err(Into::into)
    }
}

fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
}


fn icon_compare_key(icon: &[u8]) -> Result<impl PartialOrd> {
    Ok(if is_png(icon) {
        let hdr = PngHeader::from_bytes(icon)?;
        (hdr.width(), hdr.height(), 64)
    } else {
        let hdr = BitmapInfoHeader::from_bytes(icon)?;
        (hdr.width(), hdr.height(), hdr.bit_count())
    })
}

fn best_icon<'a>(mut icons: impl Iterator<Item = Result<&'a [u8]>>) -> Result<&'a [u8]> {
    let mut cur_max: &'a [u8] = if let Some(x) = icons.next() {
        x?
    } else {
        return Err(Error::NoIconFound);
    };
    let mut cur_max_key = icon_compare_key(cur_max)?;
    for icon in icons {
        let icon = icon?;
        let key = icon_compare_key(icon)?;
        if key > cur_max_key {
            cur_max = icon;
            cur_max_key = key;
        }
    }
    Ok(cur_max)
}

pub fn exelook(file_name: &CStr) -> Result<(Vec<u8>, bool, i32, i32)> {
    let map_region = FileMap::open(file_name.to_str()?)?;
    let resources = get_resources(map_region.as_ref())?;
    let (_, icon_group) = resources.group_icons().next().ok_or(Error::NoIconFound)??;
    let icons = icon_group.entries().iter().map(|ent| icon_group.image(ent.nId).map_err(Into::into));

    let best_icon = best_icon(icons)?;
    if is_png(best_icon) {
        Ok((best_icon.to_owned(), true, 0, 0))
    } else {
        let infoheader = BitmapInfoHeader::from_bytes(best_icon)?;
        if infoheader.planes() != 1 {
            return Err(Error::PlanarNotSupported);
        }
        if infoheader.compression() != 0 {
            return Err(Error::UnknownCompression);
        }
        let data = dib::decode_dib(best_icon)?;
        Ok((data, false, infoheader.width(), infoheader.height() / 2))
    }
}
