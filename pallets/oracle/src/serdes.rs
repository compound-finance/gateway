use crate::{ticker::Ticker, types::ReporterSet};
use our_std::str::FromStr;
use serde::{de, ser::SerializeSeq, Deserializer, Serialize, Serializer};

// Reporter & ReporterSet

struct ReporterSetVisitor;

impl<'de> de::Visitor<'de> for ReporterSetVisitor {
    type Value = ReporterSet;

    fn expecting(&self, formatter: &mut our_std::fmt::Formatter) -> our_std::fmt::Result {
        formatter.write_str("a vector of reporters")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: de::SeqAccess<'de>,
    {
        let mut reporters = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(r) = seq.next_element::<String>()? {
            reporters.push(
                gateway_crypto::eth_str_to_address(&r).ok_or(de::Error::custom("bad reporter"))?,
            )
        }
        Ok(ReporterSet(reporters))
    }
}

impl<'de> de::Deserialize<'de> for ReporterSet {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        de.deserialize_any(ReporterSetVisitor)
    }
}

impl Serialize for ReporterSet {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = ser.serialize_seq(Some(self.0.len()))?;
        for element in &self.0 {
            seq.serialize_element(&gateway_crypto::eth_address_string(element))?;
        }
        seq.end()
    }
}

// Ticker

struct TickerVisitor;

impl<'de> de::Visitor<'de> for TickerVisitor {
    type Value = Ticker;

    fn expecting(&self, formatter: &mut our_std::fmt::Formatter) -> our_std::fmt::Result {
        formatter.write_str("a short string representing the price ticker")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ticker::from_str(value).map_err(|_| E::custom("bad ticker"))
    }
}

impl<'de> de::Deserialize<'de> for Ticker {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        de.deserialize_any(TickerVisitor)
    }
}

impl Serialize for Ticker {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s: String = (*self).into();
        ser.serialize_str(&s)
    }
}
