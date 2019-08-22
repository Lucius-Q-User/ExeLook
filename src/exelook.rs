use std::{
    io,
    ffi::CStr,
    str::Utf8Error,
    option::NoneError,
    convert::{From, TryInto}
};

use pelite::{
    self,
    FileMap,
    pe64::{PeFile as Pe64File, Pe as Pe64},
    pe32::{PeFile as Pe32File, Pe as Pe32},
    resources::{Resources, Name, Directory}
};

use crate::{
    dib::{
        self,
        BitmapInfoHeader
    },
    icon_group::{IconGroupDirectory}
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
    let res = match Pe32File::from_bytes(bytes) {
        Ok(pe_file) => {
            pe_file.resources()
        },
        Err(pelite::Error::PeMagic) => {
            let pe_file = Pe64File::from_bytes(bytes)?;
            pe_file.resources()
        },
        Err(err) => {
            return Err(err.into());
        }
    };
    if let Err(pelite::Error::Null) = res {
        Err(Error::NoIconFound)
    } else {
        res.map_err(Into::into)
    }
}

fn get_directory<'a>(resources: &Resources<'a>, id: u32) -> Result<Directory<'a>> {
    for dentry in resources.root()?.entries() {
        if dentry.name()? == Name::Id(id) {
            return Ok(dentry.entry()?.dir()?);
        }
    }
    Err(Error::NoIconFound)
}


fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
}

fn get_icon_group<'a>(resources: &Resources<'a>) -> Result<IconGroupDirectory<'a>> {
    let icon_group_langs = get_directory(resources, 14)?.entries()
        .next()?.entry()?.dir()?;
    let icon_group = icon_group_langs.entries().next()?
        .entry()?.data()?.bytes()?;
    IconGroupDirectory::from_bytes(icon_group)
        .ok_or_else(|| pelite::Error::Bounds.into())
}

fn get_icons<'a>(resources: &Resources<'a>, names: Vec<u32>) -> Result<impl Iterator<Item = Result<&'a [u8]>>> {
    Ok(get_directory(resources, 3)?.entries().map(move |icon| {
        Ok(if let Name::Id(id) = icon.name()? {
            if names.contains(&id) {
                let data = icon.entry()?.dir()?.entries().next()?
                    .entry()?.data()?;
                Some(data.bytes()?)
            } else {
                None
            }
        } else {
            None
        })
    }).filter_map(|x| {
        match x {
            Err(w) => Some(Err(w)),
            Ok(Some(w)) => Some(Ok(w)),
            Ok(None) => None
        }
    }))
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
    let icon_ids = get_icon_group(&resources)?.entries().map(|x| u32::from(x.icon_id())).collect::<Vec<_>>();
    let best_icon = best_icon(get_icons(&resources, icon_ids)?)?;
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
