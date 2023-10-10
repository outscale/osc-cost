use std::collections::HashMap;

use icu_locid::locale;
use serde::{
    ser::{self, Impossible},
    Serialize,
};
use spreadsheet_ods::{defaultstyles::DefaultFormat, CellStyle, CellStyleRef, Sheet, WorkBook};

use super::error::{Error, Result};

#[derive(Debug)]
pub struct Serializer {
    sheets: HashMap<String, Sheet>,
    sheet_state: Option<SheetState>,
    cell_style: CellStyleRef,
}

#[derive(Debug)]
pub struct SheetState {
    sheet: Sheet,
    to_initialize: bool,
    col: u32,
    row: u32,
    name: String,
}

pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut wb = WorkBook::new(locale!("en_US"));

    let mut st_header = CellStyle::new("header", &DefaultFormat::default());
    st_header.set_font_bold();
    let cell_style_ref = wb.add_cellstyle(st_header);

    let mut serializer = Serializer {
        sheets: HashMap::new(),
        sheet_state: None,
        cell_style: cell_style_ref,
    };

    value.serialize(&mut serializer)?;

    for sheet in serializer.sheets {
        wb.push_sheet(sheet.1);
    }
    let data: Vec<u8> = Vec::new();
    spreadsheet_ods::write_ods_buf(&mut wb, data).map_err(|e| Error::Message(e.to_string()))
}

impl Serializer {
    fn serialize_value<V>(&mut self, v: V) -> Result<()>
    where
        spreadsheet_ods::Value: From<V>,
    {
        let Some(sheet_state) = &mut self.sheet_state else {
            return Err(Error::ExpectedStartStruct);
        };
        sheet_state
            .sheet
            .set_value(sheet_state.row, sheet_state.col, v);
        Ok(())
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    // The output type produced by this `Serializer` during successful
    // serialization. Most serializers that produce text or binary output should
    // set `Ok = ()` and serialize into an `io::Write` or buffer contained
    // within the `Serializer` instance, as happens here. Serializers that build
    // in-memory data structures may be simplified by using `Ok` to propagate
    // the data structure around.
    type Ok = ();

    // The error type when some error occurs during serialization.
    type Error = Error;

    // Associated types for keeping track of additional state while serializing
    // compound data structures like sequences and maps. In this case no
    // additional state is required beyond what is already stored in the
    // Serializer struct.
    type SerializeSeq = Self;
    type SerializeTuple = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeMap = Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = Self;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

    // Here we go with the simple methods. The following 12 methods receive one
    // of the primitive types of the data model and map it to JSON by appending
    // into the output string.
    fn serialize_bool(self, v: bool) -> Result<()> {
        self.serialize_value(v)
    }

    // JSON does not distinguish between different sizes of integers, so all
    // signed integers will be serialized the same and all unsigned integers
    // will be serialized the same. Other formats, especially compact binary
    // formats, may need independent logic for the different sizes.
    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_value(v)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_value(v)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_value(v)
    }

    // Not particularly efficient but this is example code anyway. A more
    // performant approach would be to use the `itoa` crate.
    fn serialize_i64(self, v: i64) -> Result<()> {
        self.serialize_value(v)
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_value(v)
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_value(v)
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_value(v)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.serialize_value(v)
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.serialize_value(v)
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.serialize_value(v)
    }

    // Serialize a char as a single-character string. Other formats may
    // represent this differently.
    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_value(v.to_string())
    }

    // This only works for strings that don't require escape sequences but you
    // get the idea. For example it would emit invalid JSON if the input string
    // contains a '"' character.
    fn serialize_str(self, v: &str) -> Result<()> {
        self.serialize_value(v)
    }

    // Serialize a byte array as an array of bytes. Could also use a base64
    // string here. Binary formats will typically represent byte arrays more
    // compactly.
    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        Err(Error::UnsupportedValue {
            kind: "bytes".to_string(),
        })
    }

    // An absent optional is represented as the JSON `null`.
    fn serialize_none(self) -> Result<()> {
        Ok(())
    }

    // A present optional is represented as just the contained value. Note that
    // this is a lossy representation. For example the values `Some(())` and
    // `None` both serialize as just `null`. Unfortunately this is typically
    // what people expect when working with JSON. Other formats are encouraged
    // to behave more intelligently if possible.
    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    // In Serde, unit means an anonymous value containing no data. Map this to
    // JSON as `null`.
    fn serialize_unit(self) -> Result<()> {
        Err(Error::UnsupportedValue {
            kind: "()".to_string(),
        })
    }

    // Unit struct means a named value containing no data. Again, since there is
    // no data, map this to JSON as `null`. There is no need to serialize the
    // name in most formats.
    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        Err(Error::UnsupportedValue {
            kind: format!("Unit Struct ({name})"),
        })
    }

    // When serializing a unit variant (or any other kind of variant), formats
    // can choose whether to keep track of it by index or by name. Binary
    // formats typically use the index of the variant and human-readable formats
    // typically use the name.
    fn serialize_unit_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        Err(Error::UnsupportedValue {
            kind: format!("Unit Variant ({name}::{variant})"),
        })
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain.
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    // Note that newtype variant (and all of the other variant serialization
    // methods) refer exclusively to the "externally tagged" enum
    // representation.
    //
    // Serialize this to JSON in externally tagged form as `{ NAME: VALUE }`.
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    // Now we get to the serialization of compound types.
    //
    // The start of the sequence, each value, and the end are three separate
    // method calls. This one is responsible only for serializing the start,
    // which in JSON is `[`.
    //
    // The length of the sequence may or may not be known ahead of time. This
    // doesn't make a difference in JSON because the length is not represented
    // explicitly in the serialized form. Some serializers may only be able to
    // support sequences for which the length is known up front.
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(self)
    }

    // Tuples look just like sequences in JSON. Some formats may be able to
    // represent tuples more efficiently by omitting the length, since tuple
    // means that the corresponding `Deserialize implementation will know the
    // length without needing to look at the serialized data.
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(Error::UnsupportedValue {
            kind: "Tuple".to_string(),
        })
    }

    // Tuple structs look just like sequences in JSON.
    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Error::UnsupportedValue {
            kind: format!("Tuple Struct ({name})"),
        })
    }

    // Tuple variants are represented in JSON as `{ NAME: [DATA...] }`. Again
    // this method is only responsible for the externally tagged representation.
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::UnsupportedValue {
            kind: format!("Tuple Variant ({name}::{variant})"),
        })
    }

    // Maps are represented in JSON as `{ K: V, K: V, ... }`.
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::UnsupportedValue {
            kind: "Map".to_string(),
        })
    }

    // Structs look just like maps in JSON. In particular, JSON requires that we
    // serialize the field names of the struct. Other formats may be able to
    // omit the field names when serializing structs because the corresponding
    // Deserialize implementation is required to know what the keys are without
    // looking at the serialized data.
    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        if self.sheet_state.is_some() {
            return Err(Error::ExpectedEndStruct);
        }

        let mut sheet: Sheet;
        let to_init: bool;
        if self.sheets.contains_key(_name) {
            sheet = self
                .sheets
                .get_mut(_name)
                .ok_or_else(|| Error::Message("Internal error".to_string()))?
                .to_owned();
            to_init = false;
        } else {
            sheet = Sheet::new(_name);
            sheet.set_header_cols(0, (len as u32) - 1);
            sheet.set_header_rows(0, 0);
            to_init = true;
        }

        let (current_row, _) = sheet.used_grid_size();
        self.sheet_state = Some(SheetState {
            sheet,
            name: String::from(_name),
            row: current_row,
            col: 0,
            to_initialize: to_init,
        });

        Ok(self)
    }

    // Struct variants are represented in JSON as `{ NAME: { K: V, ... } }`.
    // This is the externally tagged representation.
    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::UnsupportedValue {
            kind: format!("Struct Variant ({name}::{variant})"),
        })
    }
}

// The following 7 impls deal with the serialization of compound types like
// sequences and maps. Serialization of such types is begun by a Serializer
// method and followed by zero or more calls to serialize individual elements of
// the compound type and one call to end the compound type.
//
// This impl is SerializeSeq so these methods are called after `serialize_seq`
// is called on the Serializer.
impl<'a> ser::SerializeSeq for &'a mut Serializer {
    // Must match the `Ok` type of the serializer.
    type Ok = ();
    // Must match the `Error` type of the serializer.
    type Error = Error;

    // Serialize a single element of the sequence.
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    // Close the sequence.
    fn end(self) -> Result<()> {
        Ok(())
    }
}

// Structs are like maps in which the keys are constrained to be compile-time
// constant strings.
impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let Some(sheet_state) = &mut self.sheet_state else {
            return Err(Error::ExpectedStartStruct);
        };
        if sheet_state.to_initialize {
            sheet_state.sheet.set_styled_value(
                0,
                sheet_state.col,
                String::from(key),
                &self.cell_style,
            );
        }

        let res = value.serialize(&mut **self);

        let Some(sheet_state) = &mut self.sheet_state else {
            return Err(Error::ExpectedStartStruct);
        };
        sheet_state.col += 1;

        res
    }

    fn end(self) -> Result<()> {
        let Some(sheet_state) = &self.sheet_state else {
            return Err(Error::ExpectedStartStruct);
        };
        self.sheets
            .insert(sheet_state.name.clone(), sheet_state.sheet.to_owned());

        self.sheet_state = None;
        Ok(())
    }
}
