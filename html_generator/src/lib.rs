use std::fmt::{Display, Formatter};

struct SPrependToAttrs;
struct SPrependToChildren;
trait TIntoPrependToAttrsOrChildren {
    type PrependToAttrsOrChildren: TPrependToAttrsOrChildren;
}

trait TPrependToAttrsOrChildren {
    type PrependedAttrs<T, O0>;
    type PrependedChildren<T, O1>;
    fn prepend_to_attrs_or_children<T, O0, O1>(t: T, other: (O0, O1)) -> (Self::PrependedAttrs<T, O0>, Self::PrependedChildren<T, O1>);
}
impl TPrependToAttrsOrChildren for SPrependToAttrs {
    type PrependedAttrs<T, O0> = (T, O0);
    type PrependedChildren<T, O1> = O1;
    fn prepend_to_attrs_or_children<T, O0, O1>(t: T, (o0, o1): (O0, O1)) -> (Self::PrependedAttrs<T, O0>, Self::PrependedChildren<T, O1>) {
        ((t, o0), o1)
    }
}
impl TPrependToAttrsOrChildren for SPrependToChildren {
    type PrependedAttrs<T, O0> = O0;
    type PrependedChildren<T, O1> = (T, O1);
    fn prepend_to_attrs_or_children<T, O0, O1>(t: T, (o0, o1): (O0, O1)) -> (Self::PrependedAttrs<T, O0>, Self::PrependedChildren<T, O1>) {
        (o0, (t, o1))
    }
}

impl<StrKey, StrVal> TIntoPrependToAttrsOrChildren for SHtmlAttr<StrKey, StrVal> {
    type PrependToAttrsOrChildren = SPrependToAttrs;
}

impl<Attrs: THtmlAttrs, Children: THtmlChildren> TIntoPrependToAttrsOrChildren for SHtmlElement<Attrs, Children> {
    type PrependToAttrsOrChildren = SPrependToChildren;
}

trait TSplitIntoAttrsAndChildren {
    type AttrsAndChildren;
    fn split_into_attrs_and_children(self) -> Self::AttrsAndChildren;
}

impl TSplitIntoAttrsAndChildren for () {
    type AttrsAndChildren = ((), ());
    fn split_into_attrs_and_children(self) -> Self::AttrsAndChildren {
        ((), ())
    }
}

impl<IntoPrependToAttrsOrChildren: TIntoPrependToAttrsOrChildren> TSplitIntoAttrsAndChildren for IntoPrependToAttrsOrChildren {
    type AttrsAndChildren = (<IntoPrependToAttrsOrChildren::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::PrependedAttrs<Self, ()>, <IntoPrependToAttrsOrChildren::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::PrependedChildren<Self, ()>);
    fn split_into_attrs_and_children(self) -> Self::AttrsAndChildren {
        <IntoPrependToAttrsOrChildren::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::prepend_to_attrs_or_children(self, ((), ()))
    }
}

impl<IntoPrependToAttrsOrChildren: TIntoPrependToAttrsOrChildren> TSplitIntoAttrsAndChildren for (IntoPrependToAttrsOrChildren, ) {
    type AttrsAndChildren = (<IntoPrependToAttrsOrChildren::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::PrependedAttrs<Self, ()>, <IntoPrependToAttrsOrChildren::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::PrependedChildren<Self, ()>);
    fn split_into_attrs_and_children(self) -> Self::AttrsAndChildren {
        <IntoPrependToAttrsOrChildren::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::prepend_to_attrs_or_children(self, ((), ()))
    }
}

impl<Into01_0: TIntoPrependToAttrsOrChildren, Into01_1: TIntoPrependToAttrsOrChildren> TSplitIntoAttrsAndChildren for (Into01_0, Into01_1) {
    type AttrsAndChildren = (
        <Into01_0::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::PrependedAttrs<
            Into01_0,
            <Into01_1::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::PrependedAttrs<Into01_1, ()>
        >,
        <Into01_0::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::PrependedChildren<
            Into01_0,
            <Into01_1::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::PrependedChildren<Into01_1, ()>
        >
    );
    fn split_into_attrs_and_children(self) -> Self::AttrsAndChildren {
        <Into01_0::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::prepend_to_attrs_or_children(
            self.0,
            <Into01_1::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::prepend_to_attrs_or_children(self.1, ((), ()))
        )
    }
}

impl<Into01_0: TIntoPrependToAttrsOrChildren, Into01_1: TIntoPrependToAttrsOrChildren, Into01_2: TIntoPrependToAttrsOrChildren> TSplitIntoAttrsAndChildren for (Into01_0, Into01_1, Into01_2) {
    type AttrsAndChildren = (
        <Into01_0::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::PrependedAttrs<
            Into01_0,
            <Into01_1::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::PrependedAttrs<
                Into01_1,
                <Into01_2::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::PrependedAttrs<
                    Into01_2,
                    ()
                >
            >
        >,
        <Into01_0::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::PrependedChildren<
            Into01_0,
            <Into01_1::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::PrependedChildren<
                Into01_1,
                <Into01_2::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::PrependedChildren<
                    Into01_2,
                    ()
                >
            >
        >
    );
    fn split_into_attrs_and_children(self) -> Self::AttrsAndChildren {
        <Into01_0::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::prepend_to_attrs_or_children(
            self.0,
            <Into01_1::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::prepend_to_attrs_or_children(
                self.1,
                <Into01_2::PrependToAttrsOrChildren as TPrependToAttrsOrChildren>::prepend_to_attrs_or_children(
                    self.2,
                    ((), ())
                )
            )
        )
    }
}

#[test]
pub fn testme() { // TODO remove this
    dbg!((class("Test"), title("Test2"), div((class("innerdiv"), div(span("test")), div("test")))).split_into_attrs_and_children());
}

#[derive(Debug, Clone)]
pub struct SHtmlElement<Attrs: THtmlAttrs, Children: THtmlChildren> {
    str_tag_name: &'static str, // TODO impl Borrow<str>?
    attrs: Attrs,
    children: Children,
}

impl<Attrs: THtmlAttrs, Children: THtmlChildren> SHtmlElement<Attrs, Children> {
    pub fn new(str_tag_name: &'static str, attrs: Attrs, children: Children) -> Self {
        Self{str_tag_name, attrs, children}
    }
}

macro_rules! impl_element(($tag_name:ident) => {
    pub fn $tag_name<MakeAttrsAndChildren: TMakeAttrsAndChildren>(makeattrsandchildren: MakeAttrsAndChildren) -> SHtmlElement<MakeAttrsAndChildren::PrependedAttrs<()>, MakeAttrsAndChildren::PrependedChildren<()>> {
        makeattrsandchildren.prepend_to_html_element(SHtmlElement::new(
            stringify!($tag_name),
            /*attrs*/(),
            /*children*/(),
        ))
    }
});

pub trait THtmlElement : std::fmt::Display {
}

pub trait TMakeAttrsAndChildren {
    type PrependedAttrs<Attrs: THtmlAttrs>: THtmlAttrs;
    type PrependedChildren<Children: THtmlChildren>: THtmlChildren;
    fn prepend_to_html_element<Attrs: THtmlAttrs, Children: THtmlChildren>(self, htmlelement: SHtmlElement<Attrs, Children>) -> SHtmlElement<Self::PrependedAttrs<Attrs>, Self::PrependedChildren<Children>>;
}

impl_element!(table);
impl_element!(tbody);
impl_element!(tr);
impl_element!(th);
impl_element!(td);
impl_element!(div);
impl_element!(span);

pub trait THtmlAttrs {
    fn fmt_attrs(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error>;
}
pub trait THtmlChildren {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error>;
}
impl<Attrs: THtmlAttrs, Children: THtmlChildren> Display for SHtmlElement<Attrs, Children> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let str_tag_name = self.str_tag_name;
        write!(formatter, "<{str_tag_name}")?;
        self.attrs.fmt_attrs(formatter)?;
        write!(formatter, ">")?;
        self.children.fmt_children(formatter)?;
        write!(formatter, "</{str_tag_name}>")?;
        Ok(())
    }
}
impl<Attrs: THtmlAttrs, Children: THtmlChildren> THtmlElement for SHtmlElement<Attrs, Children> {
}

#[derive(Debug, Clone)]
pub struct SHtmlAttr<StrKey, StrVal>(StrKey, StrVal);
impl<StrKey: std::borrow::Borrow<str>, StrVal: std::borrow::Borrow<str>> THtmlAttrs for SHtmlAttr<StrKey, StrVal> {
    fn fmt_attrs(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(formatter, " {}=\"{}\"", self.0.borrow(), self.1.borrow())
    }
}

macro_rules! impl_attr(($attr:ident) => {
    pub fn $attr<StrVal: std::borrow::Borrow<str>>(str_val: StrVal) -> SHtmlAttr<&'static str, StrVal> {
        SHtmlAttr(stringify!($attr), str_val)
    }
});
impl_attr!(class);
impl_attr!(title);
impl_attr!(style);
impl_attr!(colspan);
impl<StrKey: std::borrow::Borrow<str>, StrVal: std::borrow::Borrow<str>> TMakeAttrsAndChildren for SHtmlAttr<StrKey, StrVal> {
    type PrependedAttrs<Attrs: THtmlAttrs> = (Self, Attrs);
    type PrependedChildren<Children: THtmlChildren> = Children;
    fn prepend_to_html_element<Attrs: THtmlAttrs, Children: THtmlChildren>(self, htmlelement: SHtmlElement<Attrs, Children>) -> SHtmlElement<Self::PrependedAttrs<Attrs>, Self::PrependedChildren<Children>> {
        let SHtmlElement { str_tag_name, attrs, children } = htmlelement;
        SHtmlElement::new(
            str_tag_name,
            (self, attrs),
            children,
        )
    }
}
impl TMakeAttrsAndChildren for () {
    type PrependedAttrs<Attrs: THtmlAttrs> = Attrs;
    type PrependedChildren<Children: THtmlChildren> = Children;
    fn prepend_to_html_element<Attrs: THtmlAttrs, Children: THtmlChildren>(self, htmlelement: SHtmlElement<Attrs, Children>) -> SHtmlElement<Self::PrependedAttrs<Attrs>, Self::PrependedChildren<Children>> {
        htmlelement
    }
}


macro_rules! nest_prepended_args(
    ($attrsorchildren:ident, $prepended:ident,) => {
        $attrsorchildren
    };
    ($attrsorchildren:ident, $prepended:ident, $t0:ident $($t:ident)*) => {
        $t0::$prepended<nest_prepended_args!($attrsorchildren, $prepended, $($t)*)>
    };
);

macro_rules! impl_make_attrs_and_children(
    ($tuple_component_0:ident $($tuple_component:ident)*) => {
        impl<$tuple_component_0: TMakeAttrsAndChildren, $($tuple_component: TMakeAttrsAndChildren,)*> TMakeAttrsAndChildren for ($tuple_component_0, $($tuple_component,)*) {
            type PrependedAttrs<Attrs: THtmlAttrs> = nest_prepended_args!(Attrs, PrependedAttrs, $tuple_component_0 $($tuple_component)*);
            type PrependedChildren<Children: THtmlChildren> = nest_prepended_args!(Children, PrependedChildren, $tuple_component_0 $($tuple_component)*);
            fn prepend_to_html_element<Attrs: THtmlAttrs, Children: THtmlChildren>(self, htmlelement: SHtmlElement<Attrs, Children>) -> SHtmlElement<Self::PrependedAttrs<Attrs>, Self::PrependedChildren<Children>> {
                #[allow(non_snake_case)]
                let ($tuple_component_0, $($tuple_component,)*) = self;
                let htmlelement = ($($tuple_component,)*).prepend_to_html_element(htmlelement);
                $tuple_component_0.prepend_to_html_element(htmlelement)
            }
        }
    }
);
impl_make_attrs_and_children!(T0);
impl_make_attrs_and_children!(T0 T1);
impl_make_attrs_and_children!(T0 T1 T2);
impl_make_attrs_and_children!(T0 T1 T2 T3);
impl_make_attrs_and_children!(T0 T1 T2 T3 T4);

impl<StrName: std::borrow::Borrow<str>, StrVal: std::borrow::Borrow<str>> THtmlAttrs for Vec<(StrName, StrVal)> {
    fn fmt_attrs(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for (str_name, str_val) in self {
            write!(formatter, " {}=\"{}\"", str_name.borrow(), str_val.borrow())?;
        }
        Ok(())
    }
}
impl<const N: usize, StrName: std::borrow::Borrow<str>, StrVal: std::borrow::Borrow<str>> THtmlAttrs for [(StrName, StrVal); N] {
    fn fmt_attrs(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for (str_name, str_val) in self {
            write!(formatter, " {}=\"{}\"", str_name.borrow(), str_val.borrow())?;
        }
        Ok(())
    }
}
impl<AttrsOuter: THtmlAttrs, ChildrenOuter: THtmlChildren> TMakeAttrsAndChildren for SHtmlElement<AttrsOuter, ChildrenOuter> {
    type PrependedAttrs<Attrs: THtmlAttrs> = Attrs;
    type PrependedChildren<Children: THtmlChildren> = (Self, Children);
    fn prepend_to_html_element<Attrs: THtmlAttrs, Children: THtmlChildren>(self, htmlelement: SHtmlElement<Attrs, Children>) -> SHtmlElement<Self::PrependedAttrs<Attrs>, Self::PrependedChildren<Children>> {
        let SHtmlElement { str_tag_name, attrs, children } = htmlelement;
        SHtmlElement::new(
            str_tag_name,
            attrs,
            (self, children),
        )
    }
}
impl<Attrs: THtmlAttrs, Children: THtmlChildren> THtmlChildren for SHtmlElement<Attrs, Children> {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt(formatter)
    }
}
impl TMakeAttrsAndChildren for &str {
    type PrependedAttrs<Attrs: THtmlAttrs> = Attrs;
    type PrependedChildren<Children: THtmlChildren> = (Self, Children);
    fn prepend_to_html_element<Attrs: THtmlAttrs, Children: THtmlChildren>(self, htmlelement: SHtmlElement<Attrs, Children>) -> SHtmlElement<Self::PrependedAttrs<Attrs>, Self::PrependedChildren<Children>> {
        let SHtmlElement { str_tag_name, attrs, children } = htmlelement;
        SHtmlElement::new(
            str_tag_name,
            attrs,
            (self, children),
        )
    }
}
impl THtmlChildren for &str {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt(formatter)
    }
}
impl TMakeAttrsAndChildren for String {
    type PrependedAttrs<Attrs: THtmlAttrs> = Attrs;
    type PrependedChildren<Children: THtmlChildren> = (Self, Children);
    fn prepend_to_html_element<Attrs: THtmlAttrs, Children: THtmlChildren>(self, htmlelement: SHtmlElement<Attrs, Children>) -> SHtmlElement<Self::PrependedAttrs<Attrs>, Self::PrependedChildren<Children>> {
        let SHtmlElement { str_tag_name, attrs, children } = htmlelement;
        SHtmlElement::new(
            str_tag_name,
            attrs,
            (self, children),
        )
    }
}
impl THtmlChildren for String {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt(formatter)
    }
}
macro_rules! impl_html_attrs_and_children_for_tuple{($($tuple_component:ident)*) => {
    impl<$($tuple_component: THtmlAttrs,)*> THtmlAttrs for ($($tuple_component,)*) {
        #[allow(unused_variables)]
        fn fmt_attrs(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            #[allow(non_snake_case)]
            let ($($tuple_component,)*) = self;
            $($tuple_component.fmt_attrs(formatter)?;)*
            Ok(())
        }
    }
    impl<$($tuple_component: THtmlChildren,)*> THtmlChildren for ($($tuple_component,)*) {
        #[allow(unused_variables)]
        fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            #[allow(non_snake_case)]
            let ($($tuple_component,)*) = self;
            $($tuple_component.fmt_children(formatter)?;)*
            Ok(())
        }
    }
}}
impl_html_attrs_and_children_for_tuple!();
impl_html_attrs_and_children_for_tuple!(T0);
impl_html_attrs_and_children_for_tuple!(T0 T1);
impl_html_attrs_and_children_for_tuple!(T0 T1 T2);
impl_html_attrs_and_children_for_tuple!(T0 T1 T2 T3);
impl_html_attrs_and_children_for_tuple!(T0 T1 T2 T3 T4);

impl<HtmlChildren: THtmlChildren> TMakeAttrsAndChildren for Vec<HtmlChildren> {
    type PrependedAttrs<Attrs: THtmlAttrs> = Attrs;
    type PrependedChildren<Children: THtmlChildren> = (Self, Children);
    fn prepend_to_html_element<Attrs: THtmlAttrs, Children: THtmlChildren>(self, htmlelement: SHtmlElement<Attrs, Children>) -> SHtmlElement<Self::PrependedAttrs<Attrs>, Self::PrependedChildren<Children>> {
        let SHtmlElement { str_tag_name, attrs, children } = htmlelement;
        SHtmlElement::new(
            str_tag_name,
            attrs,
            (self, children),
        )
    }
}
impl<HtmlChildren: THtmlChildren> THtmlChildren for Vec<HtmlChildren> {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for htmlchild in self {
            htmlchild.fmt_children(formatter)?;
        }
        Ok(())
    }
}
impl<HtmlChildren: THtmlChildren> TMakeAttrsAndChildren for Option<HtmlChildren> {
    type PrependedAttrs<Attrs: THtmlAttrs> = Attrs;
    type PrependedChildren<Children: THtmlChildren> = (Self, Children);
    fn prepend_to_html_element<Attrs: THtmlAttrs, Children: THtmlChildren>(self, htmlelement: SHtmlElement<Attrs, Children>) -> SHtmlElement<Self::PrependedAttrs<Attrs>, Self::PrependedChildren<Children>> {
        let SHtmlElement { str_tag_name, attrs, children } = htmlelement;
        SHtmlElement::new(
            str_tag_name,
            attrs,
            (self, children),
        )
    }
}
impl<HtmlChildren: THtmlChildren> THtmlChildren for Option<HtmlChildren> {
    fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        if let Some(htmlchild) = self {
            htmlchild.fmt_children(formatter)?;
        }
        Ok(())
    }
}

pub fn html_iter<Iter>(it: Iter) -> impl THtmlChildren + TMakeAttrsAndChildren
    where
        Iter: Iterator+Clone,
        Iter::Item: THtmlChildren,
{
    struct SHtmlChildrenIterator<Iter>(Iter);
    impl<Iter> THtmlChildren for SHtmlChildrenIterator<Iter>
        where
            Iter: Iterator+Clone,
            Iter::Item: THtmlChildren,
    {
        fn fmt_children(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            for htmlchild in self.0.clone() {
                htmlchild.fmt_children(formatter)?;
            }
            Ok(())
        }
    }
    impl<Iter> TMakeAttrsAndChildren for SHtmlChildrenIterator<Iter>
        where
            Iter: Iterator+Clone,
            Iter::Item: THtmlChildren,
    {
        type PrependedAttrs<Attrs: THtmlAttrs> = Attrs;
        type PrependedChildren<Children: THtmlChildren> = (Self, Children);
        fn prepend_to_html_element<Attrs: THtmlAttrs, Children: THtmlChildren>(self, htmlelement: SHtmlElement<Attrs, Children>) -> SHtmlElement<Self::PrependedAttrs<Attrs>, Self::PrependedChildren<Children>> {
            let SHtmlElement { str_tag_name, attrs, children } = htmlelement;
            SHtmlElement::new(
                str_tag_name,
                attrs,
                (self, children),
            )
        }
    }
    SHtmlChildrenIterator(it)
}
