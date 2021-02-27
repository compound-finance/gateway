use our_std::{convert::TryInto, str::FromStr};
use serde::{
    de,
    ser::{SerializeMap, SerializeSeq},
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::{
    chains::{Chain, ChainAccount, ChainAsset, Ethereum},
    symbol::Symbol,
    types::ValidatorKeys,
};

// For using in GenesisConfig / ChainSpec JSON.

// ChainAccount

struct ChainAccountVisitor;

impl<'de> de::Visitor<'de> for ChainAccountVisitor {
    type Value = ChainAccount;

    fn expecting(&self, formatter: &mut our_std::fmt::Formatter) -> our_std::fmt::Result {
        formatter.write_str("a string of the form <chain>:<address>")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        ChainAccount::from_str(value).map_err(|_| E::custom("bad account"))
    }
}

impl<'de> de::Deserialize<'de> for ChainAccount {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        de.deserialize_any(ChainAccountVisitor)
    }
}

impl Serialize for ChainAccount {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s: String = (*self).into();
        ser.serialize_str(&s)
    }
}

// ChainAsset

struct ChainAssetVisitor;

impl<'de> de::Visitor<'de> for ChainAssetVisitor {
    type Value = ChainAsset;

    fn expecting(&self, formatter: &mut our_std::fmt::Formatter) -> our_std::fmt::Result {
        formatter.write_str("a string of the form <chain>:<address>")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        ChainAsset::from_str(value).map_err(|_| E::custom("bad asset"))
    }
}

impl<'de> de::Deserialize<'de> for ChainAsset {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        de.deserialize_any(ChainAssetVisitor)
    }
}

impl Serialize for ChainAsset {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s: String = (*self).into();
        ser.serialize_str(&s)
    }
}

// Symbol

struct SymbolVisitor;

impl<'de> de::Visitor<'de> for SymbolVisitor {
    type Value = Symbol;

    fn expecting(&self, formatter: &mut our_std::fmt::Formatter) -> our_std::fmt::Result {
        formatter.write_str("a short string representing the token symbol")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Symbol::from_str(value).map_err(|_| E::custom("bad symmbol"))
    }
}

impl<'de> de::Deserialize<'de> for Symbol {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        de.deserialize_any(SymbolVisitor)
    }
}

impl Serialize for Symbol {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s: String = (*self).into();
        ser.serialize_str(&s)
    }
}

// ValidatorKeys & Vec<ValidatorKeys>

impl<'de> de::Deserialize<'de> for ValidatorKeys {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            SubstrateId,
            EthAddress,
        }
        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;
                impl<'de> de::Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(
                        &self,
                        formatter: &mut our_std::fmt::Formatter,
                    ) -> our_std::fmt::Result {
                        formatter.write_str("`substrate_id` or `eth_address`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "substrate_id" => Ok(Field::SubstrateId),
                            "eth_address" => Ok(Field::EthAddress),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct ValidatorKeysVisitor;
        impl<'de> de::Visitor<'de> for ValidatorKeysVisitor {
            type Value = ValidatorKeys;

            fn expecting(&self, formatter: &mut our_std::fmt::Formatter) -> our_std::fmt::Result {
                formatter.write_str("struct ValidatorKeys")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut substrate_id = None;
                let mut eth_address = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::SubstrateId => {
                            if substrate_id.is_some() {
                                return Err(de::Error::duplicate_field("substrate_id"));
                            }
                            let s_id: [u8; 32] = map.next_value()?;
                            substrate_id = Some(
                                s_id.try_into()
                                    .map_err(|_| de::Error::custom("bad substrate id"))?,
                            );
                        }
                        Field::EthAddress => {
                            if eth_address.is_some() {
                                return Err(de::Error::duplicate_field("eth_address"));
                            }
                            let addr: String = map.next_value()?;
                            eth_address = Some(
                                <Ethereum as Chain>::str_to_address(&addr)
                                    .map_err(|_| de::Error::custom("bad eth address"))?,
                            );
                        }
                    }
                }
                let substrate_id =
                    substrate_id.ok_or_else(|| de::Error::missing_field("substrate_id"))?;
                let eth_address =
                    eth_address.ok_or_else(|| de::Error::missing_field("eth_address"))?;
                Ok(ValidatorKeys {
                    substrate_id,
                    eth_address,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &["substrate_id", "eth_address"];
        de.deserialize_struct("ValidatorKeys", FIELDS, ValidatorKeysVisitor)
    }
}

impl Serialize for ValidatorKeys {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = ser.serialize_map(Some(2))?;
        map.serialize_key("substrate_id")?;
        map.serialize_value(&<[u8; 32]>::from(self.substrate_id.clone()))?;
        map.serialize_key("eth_address")?;
        map.serialize_value(&<Ethereum as Chain>::address_string(&self.eth_address))?;
        map.end()
    }
}
