use r53::{Name, Question, RRType};
use std::{
    cmp::{Eq, PartialEq},
    fmt::{self, Debug},
    hash::{Hash, Hasher},
};

//used as key for message and rrset cache search
pub struct EntryKey {
    name: *const Name,
    typ: RRType,
    owned: bool,
}

unsafe impl Send for EntryKey {}
unsafe impl Sync for EntryKey {}

impl EntryKey {
    pub fn new(name: Name, typ: RRType) -> Self {
        let name = Box::into_raw(Box::new(name));
        EntryKey {
            name,
            typ,
            owned: true,
        }
    }

    pub fn from_question(q: &Question) -> Self {
        EntryKey {
            name: &q.name as *const Name,
            typ: q.typ,
            owned: false,
        }
    }
}

impl Debug for EntryKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe { write!(f, "{}:{}", (*self.name), self.typ) }
    }
}

impl Hash for EntryKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe {
            (*self.name).hash(state);
        }
        state.write_u16(self.typ.as_u16());
    }
}

impl PartialEq for EntryKey {
    fn eq(&self, other: &EntryKey) -> bool {
        unsafe { self.typ == other.typ && (*self.name).eq(&(*other.name)) }
    }
}

impl Eq for EntryKey {}

impl Drop for EntryKey {
    fn drop(&mut self) {
        if self.owned {
            unsafe {
                Box::from_raw(self.name as *mut Name);
            }
        }
    }
}
