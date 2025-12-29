use serde::Serialize; // TODO Avoid Serialize here

pub trait TIsSome : Sized + Clone + std::fmt::Debug + Eq + PartialEq {
    type TypeIfTrue<T>;
    fn new_with<T>(f: impl FnOnce()->T) -> StaticOption<T, Self>;
    fn debug_fmt<T: std::fmt::Debug>(ot: &StaticOption<T, Self>, formatter: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error>;
    fn partialeq_eq<T: PartialEq>(ot0: &StaticOption<T, Self>, other: &StaticOption<T, Self>) -> bool;
    fn clone_clone<T: Clone>(ot: &StaticOption<T, Self>) -> StaticOption<T, Self>;
    fn serialize_serialize<T: Serialize, S: serde::Serializer>(ot: &StaticOption<T, Self>, serializer: S) -> Result<S::Ok, S::Error>;

    fn map<T, R>(ot: StaticOption<T, Self>, f: impl FnOnce(T)->R) -> StaticOption<R, Self>;
    fn as_ref<T>(ot: &StaticOption<T, Self>) -> StaticOption<&T, Self>;
    fn as_mut<T>(ot: &mut StaticOption<T, Self>) -> StaticOption<&mut T, Self>;
    fn map_or_else<T, R>(ot: StaticOption<T, Self>, default: impl FnOnce()->R, f: impl FnOnce(T)->R) -> R;

    fn into_option<T>(ot: StaticOption<T, Self>) -> Option<T>;
    fn tuple_2<T0, T1>(ot0: StaticOption<T0, Self>, ot1: StaticOption<T1, Self>) -> StaticOption<(T0, T1), Self>;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SIsSomeTrue;
impl TIsSome for SIsSomeTrue {
    type TypeIfTrue<T> = T;
    fn new_with<T>(f: impl FnOnce()->T) -> StaticOption<T, Self> {
        StaticOption(f())
    }
    fn debug_fmt<T: std::fmt::Debug>(ot: &StaticOption<T, Self>, formatter: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(formatter, "StaticSome({:?})", ot.0)
    }
    fn partialeq_eq<T: PartialEq>(ot0: &StaticOption<T, Self>, other: &StaticOption<T, Self>) -> bool {
        ot0.0.eq(&other.0)
    }
    fn clone_clone<T: Clone>(ot: &StaticOption<T, Self>) -> StaticOption<T, Self> {
        StaticOption(ot.0.clone())
    }
    fn serialize_serialize<T: Serialize, S: serde::Serializer>(ot: &StaticOption<T, Self>, serializer: S) -> Result<S::Ok, S::Error> {
        ot.0.serialize(serializer)
    }

    fn map<T, R>(ot: StaticOption<T, Self>, f: impl FnOnce(T)->R) -> StaticOption<R, Self> {
        StaticOption(f(ot.0))
    }
    fn as_ref<T>(ot: &StaticOption<T, Self>) -> StaticOption<&T, Self> {
        StaticOption(&ot.0)
    }
    fn as_mut<T>(ot: &mut StaticOption<T, Self>) -> StaticOption<&mut T, Self> {
        StaticOption(&mut ot.0)
    }
    fn map_or_else<T, R>(ot: StaticOption<T, Self>, _default: impl FnOnce()->R, f: impl FnOnce(T)->R) -> R {
        f(ot.0)
    }

    fn into_option<T>(ot: StaticOption<T, Self>) -> Option<T> {
        Some(ot.0)
    }
    fn tuple_2<T0, T1>(ot0: StaticOption<T0, Self>, ot1: StaticOption<T1, Self>) -> StaticOption<(T0, T1), Self> {
        StaticOption((ot0.0, ot1.0))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SIsSomeFalse;
impl TIsSome for SIsSomeFalse {
    type TypeIfTrue<T> = (); // TODO? PhantomData
    fn new_with<T>(_f: impl FnOnce()->T) -> StaticOption<T, Self> {
        StaticOption(())
    }
    fn debug_fmt<T: std::fmt::Debug>(_ot: &StaticOption<T, Self>, formatter: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(formatter, "StaticNone")
    }
    fn partialeq_eq<T: PartialEq>(_ot0: &StaticOption<T, Self>, _other: &StaticOption<T, Self>) -> bool {
        true // None==None
    }
    fn clone_clone<T>(_ot: &StaticOption<T, Self>) -> StaticOption<T, Self> {
        StaticOption(())
    }
    fn serialize_serialize<T, S: serde::Serializer>(_ot: &StaticOption<T, Self>, serializer: S) -> Result<S::Ok, S::Error> {
        None::<Self::TypeIfTrue<T>>.serialize(serializer)
    }

    fn map<T, R>(_ot: StaticOption<T, Self>, _f: impl FnOnce(T)->R) -> StaticOption<R, Self> {
        StaticOption(())
    }
    fn as_ref<T>(_ot: &StaticOption<T, Self>) -> StaticOption<&T, Self> {
        StaticOption(())
    }
    fn as_mut<T>(_ot: &mut StaticOption<T, Self>) -> StaticOption<&mut T, Self> {
        StaticOption(())
    }
    fn map_or_else<T, R>(_ot: StaticOption<T, Self>, default: impl FnOnce()->R, _f: impl FnOnce(T)->R) -> R {
        default()
    }

    fn into_option<T>(_ot: StaticOption<T, Self>) -> Option<T> {
        None
    }
    fn tuple_2<T0, T1>(_ot0: StaticOption<T0, Self>, _ot1: StaticOption<T1, Self>) -> StaticOption<(T0, T1), Self> {
        StaticOption(())
    }
}

impl<T: std::fmt::Debug, IsSome: TIsSome> std::fmt::Debug for StaticOption<T, IsSome> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        IsSome::debug_fmt(self, formatter)
    }
}

unsafe impl<T: std::marker::Send, IsSome: TIsSome> std::marker::Send for StaticOption<T, IsSome> {
}

impl <T: PartialEq, IsSome: TIsSome> PartialEq for StaticOption<T, IsSome> { // TODO Specify RHS to support None!=Some comparisons
    fn eq(&self, other: &StaticOption<T, IsSome>) -> bool {
        IsSome::partialeq_eq(self, other)
    }
}

impl <T: Eq, IsSome: TIsSome> Eq for StaticOption<T, IsSome> {
}

impl<T: Clone, IsSome: TIsSome> Clone for StaticOption<T, IsSome> {
    fn clone(&self) -> Self {
        IsSome::clone_clone(self)
    }
}

impl<T: Serialize, IsSome: TIsSome> Serialize for StaticOption<T, IsSome> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        IsSome::serialize_serialize(self, serializer)
    }
    
}

pub struct StaticOption<T, /*TODO Make this a const bool?*/IsSome: TIsSome>(IsSome::TypeIfTrue::<T>);

impl<T, IsSome: TIsSome> StaticOption<T, IsSome> {
    pub fn new_with(f: impl FnOnce()->T) -> Self {
        IsSome::new_with(f)
    }
    pub fn map<R>(self, f: impl FnOnce(T)->R) -> StaticOption<R, IsSome> {
        IsSome::map(self, f)
    }
    pub fn as_ref(&self) -> StaticOption<&T, IsSome> {
        IsSome::as_ref(self)
    }
    pub fn as_mut(&mut self) -> StaticOption<&mut T, IsSome> {
        IsSome::as_mut(self)
    }
    pub fn map_or_else<R>(self, default: impl FnOnce()->R, f: impl FnOnce(T)->R) -> R {
        IsSome::map_or_else(self, default, f)
    }

    pub fn into_option(self) -> Option<T> {
        IsSome::into_option(self)
    }
    pub fn tuple_2<T1>(self, other: StaticOption<T1, IsSome>) -> StaticOption<(T, T1), IsSome> {
        IsSome::tuple_2(self, other)
    }
}

impl<T> StaticOption<T, SIsSomeTrue> {
    pub fn unwrap_static_some(self) -> T { // TODO Should this be called "unwrap"?
        self.0
    }
}

