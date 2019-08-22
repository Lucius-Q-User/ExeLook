use std::{
    fmt,
    convert::TryInto
};

#[derive(Copy, Clone)]
pub(crate) struct IconGroupDirectory<'a> {
    bytes: &'a [u8]
}

#[derive(Copy, Clone)]
pub(crate) struct IconDirectoryEntry<'a> {
    bytes: &'a [u8]
}

struct IconDirEntryIterator<'a> {
    cur_pos: u16,
    directory: IconGroupDirectory<'a>
}

impl<'a> Iterator for IconDirEntryIterator<'a> {
    type Item = IconDirectoryEntry<'a>;
    fn next(&mut self) -> Option<IconDirectoryEntry<'a>> {
        if self.cur_pos < self.directory.num_entries() {
            let entry = self.directory.entry_at(self.cur_pos);
            self.cur_pos += 1;
            Some(entry)
        } else {
            None
        }
    }
}

impl<'a> IconDirectoryEntry<'a> {
    fn from_bytes(bytes: &'a [u8]) -> IconDirectoryEntry<'a> {
        IconDirectoryEntry { bytes }
    }
    pub(crate) fn width(&self) -> u8 {
        self.bytes[0]
    }
    pub(crate) fn height(&self) -> u8 {
        self.bytes[1]
    }
    pub(crate) fn color_count(&self) -> u8 {
        self.bytes[2]
    }
    pub(crate) fn num_planes(&self) -> u16 {
        u16::from_le_bytes(self.bytes[4..6].try_into().unwrap())
    }
    pub(crate) fn bit_count(&self) -> u16 {
        u16::from_le_bytes(self.bytes[6..8].try_into().unwrap())
    }
    pub(crate) fn num_bytes(&self) -> u32 {
        u32::from_le_bytes(self.bytes[8..12].try_into().unwrap())
    }
    pub(crate) fn icon_id(&self) -> u16 {
        u16::from_le_bytes(self.bytes[12..14].try_into().unwrap())
    }
}

impl<'a> fmt::Debug for IconGroupDirectory<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("IconGroupDirectory")
            .field("entries", &self.entries().collect::<Vec<_>>())
            .finish()
    }
}

impl<'a> fmt::Debug for IconDirectoryEntry<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("IconDirectoryEntry")
            .field("width", &self.width())
            .field("height", &self.height())
            .field("color_count", &self.color_count())
            .field("num_planes", &self.num_planes())
            .field("bit_count", &self.bit_count())
            .field("num_bytes", &self.num_bytes())
            .field("icon_id", &self.icon_id())
            .finish()
    }
}



impl<'a> IconGroupDirectory<'a> {
    pub(crate) fn from_bytes(bytes: &[u8]) -> Option<IconGroupDirectory> {
        if bytes.len() > 6 {
            let this = IconGroupDirectory { bytes };
            if this.num_entries() as usize * 14 + 6 == bytes.len() {
                Some(this)
            } else {
                None
            }
        } else {
            None
        }
    }
    pub(crate) fn entries(&self) -> impl Iterator<Item = IconDirectoryEntry> {
        IconDirEntryIterator {
            cur_pos: 0,
            directory: *self
        }
    }
    pub(crate) fn num_entries(&self) -> u16 {
        u16::from_le_bytes(self.bytes[4..6].try_into().unwrap())
    }
    pub(crate) fn entry_at(&self, idx: u16) -> IconDirectoryEntry<'a> {
        let entry_size = 14;
        let start = 6 + entry_size * idx as usize;
        let end = start + entry_size;
        let bytes = &self.bytes[start..end];
        IconDirectoryEntry::from_bytes(bytes)
    }
}
