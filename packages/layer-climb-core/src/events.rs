mod constants;
mod ibc;
pub use constants::*;
pub use ibc::*;

use crate::prelude::*;

// this wrapper (with From impls for standard event sources) helps to abstract over events to aid filtering etc.
// it is slightly opinionated in that it will search for events with the wasm- prefix if the exact type is not found
// also, it has attribute iterators to preserve performance with references
pub enum CosmosTxEvents<'a> {
    TxResponseRef(&'a layer_climb_proto::abci::TxResponse),
    TxResponseOwned(Box<layer_climb_proto::abci::TxResponse>),
    // I think this is from RPC...
    Tendermint2ListRef(&'a [tendermint::abci::Event]),
    Tendermint2ListOwned(Box<Vec<tendermint::abci::Event>>),
    CosmWasmRef(&'a [cosmwasm_std::Event]),
    CosmWasmOwned(Box<Vec<cosmwasm_std::Event>>),
}

impl<'a> From<&'a layer_climb_proto::abci::TxResponse> for CosmosTxEvents<'a> {
    fn from(resp: &'a layer_climb_proto::abci::TxResponse) -> Self {
        CosmosTxEvents::TxResponseRef(resp)
    }
}

impl From<layer_climb_proto::abci::TxResponse> for CosmosTxEvents<'static> {
    fn from(resp: layer_climb_proto::abci::TxResponse) -> Self {
        CosmosTxEvents::TxResponseOwned(Box::new(resp))
    }
}

impl<'a> From<&'a [tendermint::abci::Event]> for CosmosTxEvents<'a> {
    fn from(events: &'a [tendermint::abci::Event]) -> Self {
        CosmosTxEvents::Tendermint2ListRef(events)
    }
}

impl From<Vec<cosmwasm_std::Event>> for CosmosTxEvents<'static> {
    fn from(events: Vec<cosmwasm_std::Event>) -> Self {
        CosmosTxEvents::CosmWasmOwned(Box::new(events))
    }
}

impl<'a> From<&'a [cosmwasm_std::Event]> for CosmosTxEvents<'a> {
    fn from(events: &'a [cosmwasm_std::Event]) -> Self {
        CosmosTxEvents::CosmWasmRef(events)
    }
}

impl From<Vec<tendermint::abci::Event>> for CosmosTxEvents<'static> {
    fn from(events: Vec<tendermint::abci::Event>) -> Self {
        CosmosTxEvents::Tendermint2ListOwned(Box::new(events))
    }
}

// Local event type to allow efficient lazy converting until after filtering/finding
#[derive(Clone)]
pub enum Event<'a> {
    String(&'a layer_climb_proto::abci::StringEvent),
    // TODO - can we get rid of one of these?
    Tendermint(&'a layer_climb_proto::tendermint::Event),
    // I think this is from RPC...
    Tendermint2(&'a tendermint::abci::Event),
    CosmWasm(&'a cosmwasm_std::Event),
}

impl<'a> From<&'a layer_climb_proto::abci::StringEvent> for Event<'a> {
    fn from(event: &'a layer_climb_proto::abci::StringEvent) -> Self {
        Event::String(event)
    }
}

impl<'a> From<&'a layer_climb_proto::tendermint::Event> for Event<'a> {
    fn from(event: &'a layer_climb_proto::tendermint::Event) -> Self {
        Event::Tendermint(event)
    }
}

impl<'a> From<&'a tendermint::abci::Event> for Event<'a> {
    fn from(event: &'a tendermint::abci::Event) -> Self {
        Event::Tendermint2(event)
    }
}

impl<'a> From<&'a cosmwasm_std::Event> for Event<'a> {
    fn from(event: &'a cosmwasm_std::Event) -> Self {
        Event::CosmWasm(event)
    }
}

impl<'a> From<Event<'a>> for cosmwasm_std::Event {
    fn from(event: Event<'a>) -> cosmwasm_std::Event {
        cosmwasm_std::Event::new(event.ty()).add_attributes(event.attributes())
    }
}

impl std::fmt::Debug for Event<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            match self {
                Event::String(e) => write!(f, "{:#?}", e),
                Event::Tendermint(e) => write!(f, "{:#?}", e),
                Event::Tendermint2(e) => write!(f, "{:#?}", e),
                Event::CosmWasm(e) => write!(f, "{:#?}", e),
            }
        } else {
            match self {
                Event::String(e) => write!(f, "{:?}", e),
                Event::Tendermint(e) => write!(f, "{:?}", e),
                Event::Tendermint2(e) => write!(f, "{:?}", e),
                Event::CosmWasm(e) => write!(f, "{:?}", e),
            }
        }
    }
}

pub enum Attribute<'a> {
    String(&'a layer_climb_proto::abci::Attribute),
    // TODO - can we get rid of one of these?
    Tendermint(&'a layer_climb_proto::tendermint::EventAttribute),
    // I think this is from RPC...
    Tendermint2(&'a tendermint::abci::EventAttribute),
    CosmWasm(&'a cosmwasm_std::Attribute),
}

impl<'a> From<Attribute<'a>> for cosmwasm_std::Attribute {
    fn from(attr: Attribute<'a>) -> Self {
        cosmwasm_std::Attribute {
            key: attr.key().to_string(),
            value: attr.value().to_string(),
        }
    }
}

impl<'a> Event<'a> {
    pub fn ty(&self) -> &str {
        match self {
            Event::String(e) => &e.r#type,
            Event::Tendermint(e) => &e.r#type,
            Event::Tendermint2(e) => &e.kind,
            Event::CosmWasm(e) => &e.ty,
        }
    }

    pub fn attributes(&self) -> Box<dyn Iterator<Item = Attribute<'a>> + 'a> {
        match self {
            Event::String(e) => Box::new(e.attributes.iter().map(Attribute::String)),
            Event::Tendermint(e) => Box::new(e.attributes.iter().map(Attribute::Tendermint)),
            Event::Tendermint2(e) => Box::new(e.attributes.iter().map(Attribute::Tendermint2)),
            Event::CosmWasm(e) => Box::new(e.attributes.iter().map(Attribute::CosmWasm)),
        }
    }

    pub fn is_type(&self, ty: &str) -> bool {
        let self_ty = self.ty();

        if self_ty == ty {
            true
        } else {
            self_ty == format!("wasm-{}", ty)
        }
    }
}

impl Attribute<'_> {
    pub fn key(&self) -> &str {
        match self {
            Attribute::String(a) => &a.key,
            Attribute::Tendermint(a) => &a.key,
            Attribute::Tendermint2(a) => a.key_str().unwrap(),
            Attribute::CosmWasm(a) => &a.key,
        }
    }

    pub fn value(&self) -> &str {
        match self {
            Attribute::String(a) => &a.value,
            Attribute::Tendermint(a) => &a.value,
            Attribute::Tendermint2(a) => a.value_str().unwrap(),
            Attribute::CosmWasm(a) => &a.value,
        }
    }
}

impl<'a> CosmosTxEvents<'a> {
    pub fn events_iter(&'a self) -> Box<dyn Iterator<Item = Event<'a>> + 'a> {
        match &self {
            Self::TxResponseRef(resp) => {
                if resp.logs.len() > 1 {
                    Box::new(
                        resp.logs
                            .iter()
                            .flat_map(|log| log.events.iter().map(Event::String)),
                    )
                } else {
                    Box::new(resp.events.iter().map(Event::Tendermint))
                }
            }
            Self::TxResponseOwned(resp) => {
                if resp.logs.len() > 1 {
                    Box::new(
                        resp.logs
                            .iter()
                            .flat_map(|log| log.events.iter().map(Event::String)),
                    )
                } else {
                    Box::new(resp.events.iter().map(Event::Tendermint))
                }
            }
            Self::Tendermint2ListRef(events) => Box::new(events.iter().map(Event::Tendermint2)),
            Self::Tendermint2ListOwned(events) => Box::new(events.iter().map(Event::Tendermint2)),
            Self::CosmWasmRef(events) => Box::new(events.iter().map(Event::CosmWasm)),
            Self::CosmWasmOwned(events) => Box::new(events.iter().map(Event::CosmWasm)),
        }
    }

    pub fn filter_events_by_type<'b: 'a>(
        &'a self,
        ty: &'b str,
    ) -> impl Iterator<Item = Event<'a>> + 'a {
        self.events_iter().filter(move |e| e.is_type(ty))
    }

    pub fn filter_events_by_attr_key<'b: 'a>(
        &'a self,
        ty: &'b str,
        key: &'b str,
    ) -> impl Iterator<Item = Event<'a>> + 'a {
        self.events_iter().filter(move |e| {
            if e.is_type(ty) {
                e.attributes().any(|a| a.key() == key)
            } else {
                false
            }
        })
    }

    pub fn filter_attrs<'b: 'a>(
        &'a self,
        ty: &'b str,
        key: &'b str,
    ) -> impl Iterator<Item = Attribute<'a>> + 'a {
        self.events_iter().filter_map(move |e| {
            if e.is_type(ty) {
                e.attributes().find(|a| a.key() == key)
            } else {
                None
            }
        })
    }

    pub fn filter_map_attrs<'b: 'a, T, F>(
        &'a self,
        ty: &'b str,
        key: &'b str,
        f: F,
    ) -> impl Iterator<Item = T> + 'a
    where
        F: Clone + Fn(Attribute<'a>) -> Option<T> + 'a,
        T: 'static,
    {
        self.events_iter().filter_map(move |e| {
            if e.is_type(ty) {
                e.attributes().find(|a| a.key() == key).and_then(f.clone())
            } else {
                None
            }
        })
    }

    pub fn event_first_by_type<'b: 'a>(&'a self, ty: &'b str) -> Result<Event<'a>> {
        self.filter_events_by_type(ty)
            .next()
            .ok_or_else(|| anyhow!("couldn't find event for {}", ty))
    }

    pub fn event_first_by_attr_key<'b: 'a>(
        &'a self,
        ty: &'b str,
        key: &'b str,
    ) -> Result<Event<'a>> {
        self.filter_events_by_attr_key(ty, key)
            .next()
            .ok_or_else(|| anyhow!("couldn't find event for {}.{}", ty, key))
    }

    pub fn attr_first<'b: 'a>(&'a self, ty: &'b str, key: &'b str) -> Result<Attribute<'a>> {
        self.filter_attrs(ty, key)
            .next()
            .ok_or_else(|| anyhow!("couldn't find event attribute for {}.{}", ty, key))
    }

    pub fn map_attr_first<'b: 'a, T, F>(&'a self, ty: &'b str, key: &'b str, f: F) -> Result<T>
    where
        F: Clone + Fn(Attribute<'a>) -> Option<T> + 'a,
        T: 'static,
    {
        self.filter_map_attrs(ty, key, f)
            .next()
            .ok_or_else(|| anyhow!("couldn't find attribute for {}.{}", ty, key))
    }

    pub fn event_last_by_type<'b: 'a>(&'a self, ty: &'b str) -> Result<Event<'a>> {
        self.filter_events_by_type(ty)
            .last()
            .ok_or_else(|| anyhow!("couldn't find event for {}", ty))
    }

    pub fn event_last_by_attr_key<'b: 'a>(
        &'a self,
        ty: &'b str,
        key: &'b str,
    ) -> Result<Event<'a>> {
        self.filter_events_by_attr_key(ty, key)
            .last()
            .ok_or_else(|| anyhow!("couldn't find event for {}.{}", ty, key))
    }

    pub fn attr_last<'b: 'a>(&'a self, ty: &'b str, key: &'b str) -> Result<Attribute<'a>> {
        self.filter_attrs(ty, key)
            .last()
            .ok_or_else(|| anyhow!("couldn't find event attribute for {}.{}", ty, key))
    }

    pub fn map_attr_last<'b: 'a, T, F>(&'a self, ty: &'b str, key: &'b str, f: F) -> Result<T>
    where
        F: Clone + Fn(Attribute<'a>) -> Option<T> + 'a,
        T: 'static,
    {
        self.filter_map_attrs(ty, key, f)
            .last()
            .ok_or_else(|| anyhow!("couldn't find attribute for {}.{}", ty, key))
    }
}
