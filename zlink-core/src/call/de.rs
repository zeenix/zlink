use core::{cell::Cell, fmt, marker::PhantomData};

use serde::{
    de::{
        self, value::MapAccessDeserializer, DeserializeSeed, IntoDeserializer, MapAccess, Visitor,
    },
    Deserialize, Deserializer,
};

use super::Call;

impl<'de, M> Deserialize<'de> for Call<M>
where
    M: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CallVisitor<M>(PhantomData<M>);

        impl<'de, M> Visitor<'de> for CallVisitor<M>
        where
            M: Deserialize<'de>,
        {
            type Value = Call<M>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    "a map with optional booleans and flattened method fields"
                )
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                // 1) Prepare interior-mutable storage for optionals
                let oneway_cell = Cell::new(None);
                let more_cell = Cell::new(None);
                let upgrade_cell = Cell::new(None);

                // 2) Streaming adapter capturing booleans by Cell refs
                struct FilterMap<'a, MAcc> {
                    inner: MAcc,
                    oneway: &'a Cell<Option<bool>>,
                    more: &'a Cell<Option<bool>>,
                    upgrade: &'a Cell<Option<bool>>,
                }
                impl<'de, 'a, MAcc> MapAccess<'de> for FilterMap<'a, MAcc>
                where
                    MAcc: MapAccess<'de>,
                {
                    type Error = MAcc::Error;

                    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, MAcc::Error>
                    where
                        K: DeserializeSeed<'de>,
                    {
                        while let Some(key) = self.inner.next_key::<&str>()? {
                            match key {
                                "oneway" => {
                                    let v = self.inner.next_value()?;
                                    self.oneway.set(Some(v));
                                    continue;
                                }
                                "more" => {
                                    let v = self.inner.next_value()?;
                                    self.more.set(Some(v));
                                    continue;
                                }
                                "upgrade" => {
                                    let v = self.inner.next_value()?;
                                    self.upgrade.set(Some(v));
                                    continue;
                                }
                                other => {
                                    let de = other.into_deserializer();
                                    return seed.deserialize(de).map(Some);
                                }
                            }
                        }
                        Ok(None)
                    }

                    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, MAcc::Error>
                    where
                        V: DeserializeSeed<'de>,
                    {
                        self.inner.next_value_seed(seed)
                    }
                }

                // 3) Deserialize method: M using our FilterMap
                let filter = FilterMap {
                    inner: map,
                    oneway: &oneway_cell,
                    more: &more_cell,
                    upgrade: &upgrade_cell,
                };
                let method = M::deserialize(MapAccessDeserializer::new(filter))
                    .map_err(de::Error::custom)?;

                // 4) Extract boolean fields from Cells
                let oneway = oneway_cell.get();
                let more = more_cell.get();
                let upgrade = upgrade_cell.get();

                Ok(Call {
                    method,
                    oneway,
                    more,
                    upgrade,
                })
            }
        }

        deserializer.deserialize_map(CallVisitor(PhantomData))
    }
}
