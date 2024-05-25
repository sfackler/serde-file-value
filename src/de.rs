use std::{fmt, fs, io, path::Path};

use serde::de;

/// A deserializer which automatically reads referenced files.
///
/// Files should be referenced like `${file:/path/to/file}`.
pub struct Deserializer<'a, D, L> {
    de: D,
    listener: &'a mut L,
}

impl<'a, D, L> Deserializer<'a, D, L>
where
    L: FnMut(&Path, &io::Result<Vec<u8>>),
{
    /// Creates a new deserializer.
    ///
    /// The listener will be called on every referenced file read along with the result of the read.
    pub fn new(de: D, listener: &'a mut L) -> Self {
        Deserializer { de, listener }
    }
}

macro_rules! forward_deserialize {
    ($name:ident) => {forward_deserialize!($name, );};
    ($name:ident, $($arg:tt => $ty:ty),*) => {
        fn $name<V>(self, $($arg: $ty,)* visitor: V) -> Result<V::Value, D::Error>
            where V: de::Visitor<'de>
        {
            let visitor = Visitor {
                visitor,
                listener: self.listener,
            };
            self.de.$name($($arg,)* visitor)
        }
    }
}

impl<'a, 'de, D, L> de::Deserializer<'de> for Deserializer<'a, D, L>
where
    D: de::Deserializer<'de>,
    L: FnMut(&Path, &io::Result<Vec<u8>>),
{
    type Error = D::Error;

    forward_deserialize!(deserialize_any);
    forward_deserialize!(deserialize_bool);
    forward_deserialize!(deserialize_u8);
    forward_deserialize!(deserialize_u16);
    forward_deserialize!(deserialize_u32);
    forward_deserialize!(deserialize_u64);
    forward_deserialize!(deserialize_i8);
    forward_deserialize!(deserialize_i16);
    forward_deserialize!(deserialize_i32);
    forward_deserialize!(deserialize_i64);
    forward_deserialize!(deserialize_f32);
    forward_deserialize!(deserialize_f64);
    forward_deserialize!(deserialize_char);
    forward_deserialize!(deserialize_str);
    forward_deserialize!(deserialize_string);
    forward_deserialize!(deserialize_unit);
    forward_deserialize!(deserialize_option);
    forward_deserialize!(deserialize_seq);
    forward_deserialize!(deserialize_bytes);
    forward_deserialize!(deserialize_byte_buf);
    forward_deserialize!(deserialize_map);
    forward_deserialize!(deserialize_unit_struct, name => &'static str);
    forward_deserialize!(deserialize_newtype_struct, name => &'static str);
    forward_deserialize!(deserialize_tuple_struct, name => &'static str, len => usize);
    forward_deserialize!(deserialize_struct,
                         name => &'static str,
                         fields => &'static [&'static str]);
    forward_deserialize!(deserialize_identifier);
    forward_deserialize!(deserialize_tuple, len => usize);
    forward_deserialize!(deserialize_enum,
                         name => &'static str,
                         variants => &'static [&'static str]);
    forward_deserialize!(deserialize_ignored_any);
}

struct Visitor<'a, V, L> {
    visitor: V,
    listener: &'a mut L,
}

macro_rules! forward_visit {
    ($name:ident, $ty:ty) => {
        fn $name<E>(self, v: $ty) -> Result<V::Value, E>
        where
            E: de::Error,
        {
            self.visitor.$name(v)
        }
    };
}

impl<V, L> Visitor<'_, V, L>
where
    L: FnMut(&Path, &io::Result<Vec<u8>>),
{
    fn expand_str<E>(&mut self, s: &str) -> Result<Option<String>, E>
    where
        E: de::Error,
    {
        match s.strip_prefix("${file:").and_then(|s| s.strip_suffix('}')) {
            Some(path) => {
                let value = fs::read(path);
                (self.listener)(path.as_ref(), &value);
                match value {
                    Ok(contents) => {
                        let contents = String::from_utf8(contents).map_err(|e| {
                            E::custom(format_args!("error parsing file {path}: {e}"))
                        })?;
                        Ok(Some(contents))
                    }
                    Err(e) => Err(E::custom(format_args!("error reading file {path}: {e}"))),
                }
            }
            None => Ok(None),
        }
    }
}

impl<'de, V, L> de::Visitor<'de> for Visitor<'_, V, L>
where
    V: de::Visitor<'de>,
    L: FnMut(&Path, &io::Result<Vec<u8>>),
{
    type Value = V::Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        self.visitor.expecting(formatter)
    }

    forward_visit!(visit_bool, bool);
    forward_visit!(visit_i8, i8);
    forward_visit!(visit_i16, i16);
    forward_visit!(visit_i32, i32);
    forward_visit!(visit_i64, i64);
    forward_visit!(visit_u8, u8);
    forward_visit!(visit_u16, u16);
    forward_visit!(visit_u32, u32);
    forward_visit!(visit_u64, u64);
    forward_visit!(visit_f32, f32);
    forward_visit!(visit_f64, f64);
    forward_visit!(visit_char, char);
    forward_visit!(visit_bytes, &[u8]);
    forward_visit!(visit_byte_buf, Vec<u8>);

    fn visit_str<E>(mut self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match self.expand_str(v)? {
            Some(s) => self.visitor.visit_string(s),
            None => self.visitor.visit_str(v),
        }
    }

    fn visit_string<E>(mut self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match self.expand_str(&v)? {
            Some(s) => self.visitor.visit_string(s),
            None => self.visitor.visit_string(v),
        }
    }

    fn visit_borrowed_str<E>(mut self, v: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match self.expand_str(v)? {
            Some(s) => self.visitor.visit_string(s),
            None => self.visitor.visit_borrowed_str(v),
        }
    }

    fn visit_unit<E>(self) -> Result<V::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_unit()
    }

    fn visit_none<E>(self) -> Result<V::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_none()
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let deserializer = Deserializer {
            de: deserializer,
            listener: self.listener,
        };
        self.visitor.visit_some(deserializer)
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let deserializer = Deserializer {
            de: deserializer,
            listener: self.listener,
        };
        self.visitor.visit_newtype_struct(deserializer)
    }

    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let visitor = Visitor {
            visitor: seq,
            listener: self.listener,
        };
        self.visitor.visit_seq(visitor)
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        let visitor = Visitor {
            visitor: map,
            listener: self.listener,
        };
        self.visitor.visit_map(visitor)
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        let visitor = Visitor {
            visitor: data,
            listener: self.listener,
        };
        self.visitor.visit_enum(visitor)
    }
}

impl<'de, V, L> de::SeqAccess<'de> for Visitor<'_, V, L>
where
    V: de::SeqAccess<'de>,
    L: FnMut(&Path, &io::Result<Vec<u8>>),
{
    type Error = V::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        let seed = DeserializeSeed {
            seed,
            listener: self.listener,
        };
        self.visitor.next_element_seed(seed)
    }

    fn size_hint(&self) -> Option<usize> {
        self.visitor.size_hint()
    }
}

impl<'de, V, L> de::MapAccess<'de> for Visitor<'_, V, L>
where
    V: de::MapAccess<'de>,
    L: FnMut(&Path, &io::Result<Vec<u8>>),
{
    type Error = V::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        let seed = DeserializeSeed {
            seed,
            listener: self.listener,
        };
        self.visitor.next_key_seed(seed)
    }

    fn next_value_seed<S>(&mut self, seed: S) -> Result<S::Value, Self::Error>
    where
        S: de::DeserializeSeed<'de>,
    {
        let seed = DeserializeSeed {
            seed,
            listener: self.listener,
        };
        self.visitor.next_value_seed(seed)
    }

    fn size_hint(&self) -> Option<usize> {
        self.visitor.size_hint()
    }
}

impl<'a, 'de, V, L> de::EnumAccess<'de> for Visitor<'a, V, L>
where
    V: de::EnumAccess<'de>,
    L: FnMut(&Path, &io::Result<Vec<u8>>),
{
    type Error = V::Error;

    type Variant = Visitor<'a, V::Variant, L>;

    fn variant_seed<S>(self, seed: S) -> Result<(S::Value, Self::Variant), Self::Error>
    where
        S: de::DeserializeSeed<'de>,
    {
        let seed = DeserializeSeed {
            seed,
            listener: self.listener,
        };
        match self.visitor.variant_seed(seed) {
            Ok((value, variant)) => {
                let variant = Visitor {
                    visitor: variant,
                    listener: self.listener,
                };
                Ok((value, variant))
            }
            Err(e) => Err(e),
        }
    }
}

impl<'de, V, L> de::VariantAccess<'de> for Visitor<'_, V, L>
where
    V: de::VariantAccess<'de>,
    L: FnMut(&Path, &io::Result<Vec<u8>>),
{
    type Error = V::Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        self.visitor.unit_variant()
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        let seed = DeserializeSeed {
            seed,
            listener: self.listener,
        };
        self.visitor.newtype_variant_seed(seed)
    }

    fn tuple_variant<V2>(self, len: usize, visitor: V2) -> Result<V2::Value, Self::Error>
    where
        V2: de::Visitor<'de>,
    {
        let visitor = Visitor {
            visitor,
            listener: self.listener,
        };
        self.visitor.tuple_variant(len, visitor)
    }

    fn struct_variant<V2>(
        self,
        fields: &'static [&'static str],
        visitor: V2,
    ) -> Result<V2::Value, Self::Error>
    where
        V2: de::Visitor<'de>,
    {
        let visitor = Visitor {
            visitor,
            listener: self.listener,
        };
        self.visitor.struct_variant(fields, visitor)
    }
}

struct DeserializeSeed<'a, S, L> {
    seed: S,
    listener: &'a mut L,
}

impl<'de, S, L> de::DeserializeSeed<'de> for DeserializeSeed<'_, S, L>
where
    S: de::DeserializeSeed<'de>,
    L: FnMut(&Path, &io::Result<Vec<u8>>),
{
    type Value = S::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let deserializer = Deserializer {
            de: deserializer,
            listener: self.listener,
        };
        self.seed.deserialize(deserializer)
    }
}
