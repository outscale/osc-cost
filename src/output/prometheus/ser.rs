use prometheus::core::GenericGauge;
use prometheus::Gauge;
use prometheus::Opts;
use prometheus::Registry;
use prometheus::{core::AtomicF64, Encoder, TextEncoder};
use serde::{
    ser::{self, Impossible},
    Serialize,
};
use std::collections::HashMap;

use super::error::{Error, Result};

#[derive(Debug)]
struct Serializer {
    gauges: HashMap<String, Vec<GenericGauge<AtomicF64>>>,
    gauge_state: Option<GaugeState>,
    filter_label: FilterLabel,
}
#[derive(Debug)]
struct FilterLabel {
    include: Vec<String>,
    primary: CustomLabelKey,
    secondary: CustomLabelKey,
    label_name: String,
}

#[derive(Debug)]
pub struct CustomLabelKey {
    pub name: String,
    pub help: String,
    pub key: String,
}

#[derive(Debug)]
struct GaugeState {
    label_key: String,
    primary_value: String,
    secondary_value: String,
    gauge_primary_opt: Opts,
    gauge_secondary_opt: Opts,
    name: String,
}
pub fn to_prom<T>(
    value: &T,
    include: Vec<String>,
    primary: CustomLabelKey,
    secondary: CustomLabelKey,
    label_name: String,
) -> Result<String>
where
    T: Serialize,
{
    let registry = Registry::new();
    let filter_label = FilterLabel {
        include,
        primary,
        secondary,
        label_name,
    };
    let mut serializer = Serializer {
        gauges: HashMap::new(),
        gauge_state: None,
        filter_label,
    };
    value.serialize(&mut serializer)?;

    for gauge_list in serializer.gauges {
        for gauge in gauge_list.1 {
            registry.register(Box::new(gauge)).unwrap_or_default();
        }
    }
    let mut buffer = Vec::<u8>::new();
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    let Ok(_) = encoder.encode(&metric_families, &mut buffer) else {
        return Err(Error::Message("Can not encode ".to_string()));
    };
    Ok(String::from_utf8(buffer.clone()).unwrap_or_default())
}

impl Serializer {
    fn serialize_value<V>(&mut self, v: V) -> Result<()>
    where
        V: ToString,
    {
        let Some(gauge_state) = &mut self.gauge_state else {
            return Err(Error::ExpectedStartStruct);
        };
        let primary_key = self.filter_label.primary.key.to_string();
        let secondary_key = self.filter_label.secondary.key.to_string();
        let label_name = self.filter_label.label_name.to_string();
        let label_key = &gauge_state.label_key;
        match label_key {
            label_key if *label_key == primary_key => gauge_state.primary_value = v.to_string(),
            label_key if *label_key == secondary_key => gauge_state.secondary_value = v.to_string(),
            &_ => {
                if *label_key == label_name {
                    gauge_state.name = gauge_state.name.clone() + &v.to_string()
                }

                let gauge_primary_opt = gauge_state
                    .gauge_primary_opt
                    .clone()
                    .const_label(gauge_state.label_key.clone(), v.to_string());

                let gauge_secondary_opt = gauge_state
                    .gauge_secondary_opt
                    .clone()
                    .const_label(gauge_state.label_key.clone(), v.to_string());

                gauge_state.gauge_primary_opt = gauge_primary_opt;
                gauge_state.gauge_secondary_opt = gauge_secondary_opt;
            }
        }

        Ok(())
    }
}

impl ser::Serializer for &mut Serializer {
    type Ok = ();

    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeMap = Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = Self;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.serialize_value(v)
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_value(v)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_value(v)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_value(v)
    }

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

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_value(v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.serialize_value(v)
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<()> {
        Err(Error::UnsupportedValue {
            kind: "bytes".to_string(),
        })
    }

    fn serialize_none(self) -> Result<()> {
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        Err(Error::UnsupportedValue {
            kind: "()".to_string(),
        })
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        Err(Error::UnsupportedValue {
            kind: format!("Unit Struct ({name})"),
        })
    }

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

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

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

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(Error::UnsupportedValue {
            kind: "Tuple".to_string(),
        })
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Error::UnsupportedValue {
            kind: format!("Tuple Struct ({name})"),
        })
    }

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

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::UnsupportedValue {
            kind: "Map".to_string(),
        })
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        if self.gauge_state.is_some() {
            return Err(Error::ExpectedEndStruct);
        }

        let gauge_primary_opt = Opts::new(
            String::from(_name) + &self.filter_label.primary.name.to_owned(),
            String::from(_name) + &self.filter_label.primary.help.to_owned(),
        );

        let gauge_secondary_opt = Opts::new(
            String::from(_name) + &self.filter_label.secondary.name.to_owned(),
            String::from(_name) + &self.filter_label.secondary.help.to_owned(),
        );
        self.gauge_state = Some(GaugeState {
            label_key: "".to_string(),
            primary_value: "".to_string(),
            secondary_value: "".to_string(),
            gauge_primary_opt,
            gauge_secondary_opt,
            name: String::from(_name),
        });

        Ok(self)
    }

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

    fn collect_str<T>(self, value: &T) -> std::result::Result<Self::Ok, Self::Error>
    where
        T: std::fmt::Display + ?Sized,
    {
        self.serialize_str(&value.to_string())
    }

    fn is_human_readable(&self) -> bool {
        true
    }

    fn serialize_i128(self, _value: i128) -> std::result::Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedValue {
            kind: "i128".to_string(),
        })
    }

    fn serialize_u128(self, _value: u128) -> std::result::Result<Self::Ok, Self::Error> {
        Err(Error::UnsupportedValue {
            kind: "u128".to_string(),
        })
    }
}

impl ser::SerializeStruct for &mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let Some(gauge_state) = &mut self.gauge_state else {
            return Err(Error::ExpectedStartStruct);
        };
        if self.filter_label.include.iter().any(|e| key.contains(e)) {
            gauge_state.label_key = key.to_string();
            value.serialize(&mut **self)?;
        }
        Ok(())
    }

    fn end(self) -> Result<()> {
        let Some(gauge_state) = &self.gauge_state else {
            return Err(Error::ExpectedStartStruct);
        };
        let Ok(gauge_primary) = Gauge::with_opts(gauge_state.gauge_primary_opt.clone()) else {
            return Err(Error::Message("Can not create gauge_primary".to_string()));
        };
        let Ok(gauge_secondary) = Gauge::with_opts(gauge_state.gauge_secondary_opt.clone()) else {
            return Err(Error::Message("Can not create gauge_secondary".to_string()));
        };
        gauge_primary.add(gauge_state.primary_value.parse::<f64>().unwrap_or_default());
        gauge_secondary.add(
            gauge_state
                .secondary_value
                .parse::<f64>()
                .unwrap_or_default(),
        );

        let gauge = vec![gauge_primary, gauge_secondary];
        self.gauges.insert(gauge_state.name.to_owned(), gauge);

        self.gauge_state = None;
        Ok(())
    }
}

impl ser::SerializeSeq for &mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}
