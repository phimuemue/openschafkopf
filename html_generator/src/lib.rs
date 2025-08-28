use std::fmt::{Display, Formatter};

pub struct IsAttribute;
pub struct IsChild;
pub trait AttributeOrChild {
    type Attribute: AttributeOrChild;
    type Child: AttributeOrChild;
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child);
    fn fmt_attribute_or_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error>;
}

impl AttributeOrChild for () {
    type Attribute = ();
    type Child = ();
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
        ((), ())
    }
    fn fmt_attribute_or_child(&self, _formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        Ok(())
    }
}

macro_rules! impl_attributeorchild_for_tuple {($($t:ident)*) => {
    impl<$($t: AttributeOrChild,)*> AttributeOrChild for ($($t,)*) {
        type Attribute = ($($t::Attribute,)*);
        type Child = ($($t::Child,)*);
        fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
            #[allow(non_snake_case)]
            let ($($t,)*) = self;
            #[allow(non_snake_case)]
            let ($($t,)*) = ($($t.split_into_attributes_and_children(),)*);
            (
                ($($t.0,)*),
                ($($t.1,)*),
            )
        }
        #[allow(unused_variables)]
        fn fmt_attribute_or_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            #[allow(non_snake_case)]
            let ($($t,)*) = self;
            $($t.fmt_attribute_or_child(formatter)?;)*
            Ok(())
        }
    }
}}
impl_attributeorchild_for_tuple!(T0);
impl_attributeorchild_for_tuple!(T0 T1);
impl_attributeorchild_for_tuple!(T0 T1 T2);
impl_attributeorchild_for_tuple!(T0 T1 T2 T3);
impl_attributeorchild_for_tuple!(T0 T1 T2 T3 T4);


impl<StrKey: std::borrow::Borrow<str>, StrVal: std::borrow::Borrow<str>> AttributeOrChild for SHtmlAttr<StrKey, StrVal> {
    type Attribute = Self;
    type Child = ();
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
        (self, ())
    }
    fn fmt_attribute_or_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(formatter, " {}=\"{}\"", self.0.borrow(), self.1.borrow())
    }
}

impl<AoC: AttributeOrChild> AttributeOrChild for HtmlElement<AoC> {
    type Attribute = ();
    type Child = Self;
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
        ((), self)
    }
    fn fmt_attribute_or_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let tag_name = self.tag_name;
        write!(formatter, "<{tag_name}")?;
        self.attributes.fmt_attribute_or_child(formatter)?;
        write!(formatter, ">")?;
        self.children.fmt_attribute_or_child(formatter)?;
        write!(formatter, "</{tag_name}>")?;
        Ok(())
    }
}
impl<AoC: AttributeOrChild> AttributeOrChild for VoidElement<AoC> {
    type Attribute = ();
    type Child = Self;
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
        ((), self)
    }
    fn fmt_attribute_or_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let tag_name = self.tag_name;
        write!(formatter, "<{tag_name}")?;
        self.attributes.fmt_attribute_or_child(formatter)?;
        write!(formatter, "/>")?;
        Ok(())
    }
}

impl AttributeOrChild for &str {
    type Attribute = ();
    type Child = Self;
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
        ((), self)
    }
    fn fmt_attribute_or_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt(formatter)
    }
}

impl AttributeOrChild for String {
    type Attribute = ();
    type Child = Self;
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
        ((), self)
    }
    fn fmt_attribute_or_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt(formatter)
    }
}

#[test]
pub fn testme() { // TODO remove this
    dbg!((class("Test"), Some(attributes::title("Test2")), div((class("innerdiv"), div(Some(elements::span("test"))), div("test")))).split_into_attributes_and_children());
    dbg!(((class("Test"), class("test2")), Some(attributes::title("Test2")), div((class("innerdiv"), div(Some(elements::span("test"))), div("test")))).split_into_attributes_and_children());
    dbg!(((class("Test"), class("test2")), Some(attributes::title("Test2")), div((class("innerdiv"), div((Some(elements::span("test")), "nochmal text", br(()), "test after newline")), div("test")))).split_into_attributes_and_children());

    assert_eq!(
        div((
            class("DivClass"),
            id("DivId"),
            p("This is the first paragraph."),
            p((
                "Second paragraph contains a ",
                a((href("www.example.com"), "link")),
                " and",
                br(()),
                "a linebreak."
            )),
        )).to_string(),
        r#"<div class="DivClass" id="DivId"><p>This is the first paragraph.</p><p>Second paragraph contains a <a href="www.example.com">link</a> and<br/>a linebreak.</p></div>"#,
    );
}

#[derive(Debug, Clone)]
pub struct HtmlElement<AoC: AttributeOrChild> {
    tag_name: &'static str, // TODO impl Borrow<str>?
    attributes: AoC::Attribute,
    children: AoC::Child,
}

impl<AoC: AttributeOrChild> HtmlElement<AoC> {
    pub fn new(tag_name: &'static str, attributeorchild: AoC) -> Self {
        let (attributes, children) = attributeorchild.split_into_attributes_and_children();
        Self{tag_name, attributes, children}
    }
}

#[derive(Debug, Clone)]
pub struct VoidElement<A>
    where (A, ()): AttributeOrChild
{
    tag_name: &'static str, // TODO impl Borrow<str>?
    attributes: A,
}

impl<A> VoidElement<A>
    where (A, ()): AttributeOrChild
{
    pub fn new(tag_name: &'static str, attributes: A) -> Self {
        Self{tag_name, attributes}
    }
}

pub mod elements {
    use super::*;
    macro_rules! impl_element(($tag_name:ident) => {
        pub fn $tag_name<AoC: AttributeOrChild>(attributes_and_children: AoC) -> HtmlElement<AoC> {
            HtmlElement::new(stringify!($tag_name), attributes_and_children)
        }
    });
    macro_rules! impl_void(($tag_name:ident) => {
        pub fn $tag_name<A>(attributes: A) -> VoidElement<A>
            where
                (A, ()): AttributeOrChild,
        {
            VoidElement::new(stringify!($tag_name), attributes)
        }
    });

    // Scraped from https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements, https://developer.mozilla.org/en-US/docs/Glossary/Void_element
    impl_element!(a);
    impl_element!(abbr);
    impl_element!(acronym);
    impl_element!(address);
    impl_void!(area);
    impl_element!(article);
    impl_element!(aside);
    impl_element!(audio);
    impl_element!(b);
    impl_void!(base);
    impl_element!(bdi);
    impl_element!(bdo);
    impl_element!(big);
    impl_element!(blockquote);
    impl_element!(body);
    impl_void!(br);
    impl_element!(button);
    impl_element!(canvas);
    impl_element!(caption);
    impl_element!(center);
    impl_element!(cite);
    impl_element!(code);
    impl_void!(col);
    impl_element!(colgroup);
    impl_element!(data);
    impl_element!(datalist);
    impl_element!(dd);
    impl_element!(del);
    impl_element!(details);
    impl_element!(dfn);
    impl_element!(dialog);
    impl_element!(dir);
    impl_element!(div);
    impl_element!(dl);
    impl_element!(dt);
    impl_element!(em);
    impl_void!(embed);
    impl_element!(fencedframe);
    impl_element!(fieldset);
    impl_element!(figcaption);
    impl_element!(figure);
    impl_element!(font);
    impl_element!(footer);
    impl_element!(form);
    impl_element!(frame);
    impl_element!(frameset);
    impl_element!(h1);
    impl_element!(head);
    impl_element!(header);
    impl_element!(hgroup);
    impl_void!(hr);
    impl_element!(html);
    impl_element!(i);
    impl_element!(iframe);
    impl_void!(img);
    impl_void!(input);
    impl_element!(ins);
    impl_element!(kbd);
    impl_element!(label);
    impl_element!(legend);
    impl_element!(li);
    impl_void!(link);
    impl_element!(main);
    impl_element!(map);
    impl_element!(mark);
    impl_element!(marquee);
    impl_element!(menu);
    impl_void!(meta);
    impl_element!(meter);
    impl_element!(nav);
    impl_element!(nobr);
    impl_element!(noembed);
    impl_element!(noframes);
    impl_element!(noscript);
    impl_element!(object);
    impl_element!(ol);
    impl_element!(optgroup);
    impl_element!(option);
    impl_element!(output);
    impl_element!(p);
    impl_void!(param);
    impl_element!(picture);
    impl_element!(plaintext);
    impl_element!(pre);
    impl_element!(progress);
    impl_element!(q);
    impl_element!(rb);
    impl_element!(rp);
    impl_element!(rt);
    impl_element!(rtc);
    impl_element!(ruby);
    impl_element!(s);
    impl_element!(samp);
    impl_element!(script);
    impl_element!(search);
    impl_element!(section);
    impl_element!(select);
    impl_element!(selectedcontent);
    impl_element!(slot);
    impl_element!(small);
    impl_void!(source);
    impl_element!(span);
    impl_element!(strike);
    impl_element!(strong);
    impl_element!(style);
    impl_element!(sub);
    impl_element!(summary);
    impl_element!(sup);
    impl_element!(table);
    impl_element!(tbody);
    impl_element!(td);
    impl_element!(template);
    impl_element!(textarea);
    impl_element!(tfoot);
    impl_element!(th);
    impl_element!(thead);
    impl_element!(time);
    impl_element!(title);
    impl_element!(tr);
    impl_void!(track);
    impl_element!(tt);
    impl_element!(u);
    impl_element!(ul);
    impl_element!(var);
    impl_element!(video);
    impl_void!(wbr);
    impl_element!(xmp);
}
#[allow(ambiguous_glob_reexports)]
pub use elements::*;

impl<AoC: AttributeOrChild> Display for HtmlElement<AoC> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt_attribute_or_child(formatter)
    }
}
impl<AoC: AttributeOrChild> Display for VoidElement<AoC> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.fmt_attribute_or_child(formatter)
    }
}
pub fn html_display(t: impl AttributeOrChild) -> impl Display {
    struct SDisplay<T: AttributeOrChild>(T);
    impl<AoC: AttributeOrChild> Display for SDisplay<AoC> {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            self.0.fmt_attribute_or_child(formatter)
        }
    }
    SDisplay(t)
}

#[derive(Debug, Clone)]
pub struct SHtmlAttr<StrKey, StrVal>(StrKey, StrVal);

pub mod attributes {
    use super::*;
    macro_rules! impl_attr(($attr:ident) => {
        pub fn $attr<StrVal: std::borrow::Borrow<str>>(str_val: StrVal) -> SHtmlAttr<&'static str, StrVal> {
            SHtmlAttr(stringify!($attr), str_val)
        }
    });
    impl_attr!(accept);
    impl_attr!(accept_charset);
    impl_attr!(accesskey);
    impl_attr!(action);
    impl_attr!(align);
    impl_attr!(allow);
    impl_attr!(alt);
    impl_attr!(as_); // "as" is a Rust keyword
    impl_attr!(async_);
    impl_attr!(autocapitalize);
    impl_attr!(autocomplete);
    impl_attr!(autoplay);
    impl_attr!(background);
    impl_attr!(bgcolor);
    impl_attr!(border);
    impl_attr!(capture);
    impl_attr!(charset);
    impl_attr!(checked);
    impl_attr!(cite);
    impl_attr!(class);
    impl_attr!(color);
    impl_attr!(cols);
    impl_attr!(colspan);
    impl_attr!(content);
    impl_attr!(contenteditable);
    impl_attr!(controls);
    impl_attr!(coords);
    impl_attr!(crossorigin);
    impl_attr!(csp);
    impl_attr!(data);
    impl_attr!(data_star); // asterisk cannot be used
    impl_attr!(datetime);
    impl_attr!(decoding);
    impl_attr!(default);
    impl_attr!(defer);
    impl_attr!(dir);
    impl_attr!(dirname);
    impl_attr!(disabled);
    impl_attr!(download);
    impl_attr!(draggable);
    impl_attr!(enctype);
    impl_attr!(enterkeyhint);
    impl_attr!(elementtiming);
    impl_attr!(for_); // "for" is a Rust keyword
    impl_attr!(form);
    impl_attr!(formaction);
    impl_attr!(formenctype);
    impl_attr!(formmethod);
    impl_attr!(formnovalidate);
    impl_attr!(formtarget);
    impl_attr!(headers);
    impl_attr!(height);
    impl_attr!(hidden);
    impl_attr!(high);
    impl_attr!(href);
    impl_attr!(hreflang);
    impl_attr!(http_equiv);
    impl_attr!(id);
    impl_attr!(integrity);
    impl_attr!(inputmode);
    impl_attr!(ismap);
    impl_attr!(itemprop);
    impl_attr!(kind);
    impl_attr!(label);
    impl_attr!(lang);
    impl_attr!(language);
    impl_attr!(loading);
    impl_attr!(list);
    impl_attr!(loop_); // "loop" is a Rust keyword
    impl_attr!(low);
    impl_attr!(max);
    impl_attr!(maxlength);
    impl_attr!(minlength);
    impl_attr!(media);
    impl_attr!(method);
    impl_attr!(min);
    impl_attr!(multiple);
    impl_attr!(muted);
    impl_attr!(name);
    impl_attr!(novalidate);
    impl_attr!(open);
    impl_attr!(optimum);
    impl_attr!(pattern);
    impl_attr!(ping);
    impl_attr!(placeholder);
    impl_attr!(playsinline);
    impl_attr!(poster);
    impl_attr!(preload);
    impl_attr!(readonly);
    impl_attr!(referrerpolicy);
    impl_attr!(rel);
    impl_attr!(required);
    impl_attr!(reversed);
    impl_attr!(role);
    impl_attr!(rows);
    impl_attr!(rowspan);
    impl_attr!(sandbox);
    impl_attr!(scope);
    impl_attr!(selected);
    impl_attr!(shape);
    impl_attr!(size);
    impl_attr!(sizes);
    impl_attr!(slot);
    impl_attr!(span);
    impl_attr!(spellcheck);
    impl_attr!(src);
    impl_attr!(srcdoc);
    impl_attr!(srclang);
    impl_attr!(srcset);
    impl_attr!(start);
    impl_attr!(step);
    impl_attr!(style);
    impl_attr!(summary);
    impl_attr!(tabindex);
    impl_attr!(target);
    impl_attr!(title);
    impl_attr!(translate);
    impl_attr!(type_); // "type" is a Rust keyword
    impl_attr!(usemap);
    impl_attr!(value);
    impl_attr!(width);
    impl_attr!(wrap);
}
#[allow(ambiguous_glob_reexports)]
pub use attributes::*;

impl<AoC: AttributeOrChild> AttributeOrChild for Vec<AoC> {
    type Attribute = Vec<AoC::Attribute>;
    type Child = Vec<AoC::Child>;
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
        let mut vecattribute = Vec::new();
        let mut vecchild = Vec::new();
        for attributeorchild in self {
            let (attribute, child) = attributeorchild.split_into_attributes_and_children();
            vecattribute.push(attribute);
            vecchild.push(child);
        }
        (vecattribute, vecchild)
    }
    fn fmt_attribute_or_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for attributeorchild in self {
            attributeorchild.fmt_attribute_or_child(formatter)?;
        }
        Ok(())
    }
}
impl<AoC: AttributeOrChild> AttributeOrChild for Option<AoC> {
    type Attribute = Option<AoC::Attribute>;
    type Child = Option<AoC::Child>;
    fn split_into_attributes_and_children(self) -> (Self::Attribute, Self::Child) {
        if let Some(attributeorchild) = self {
            let (attribute, child) = attributeorchild.split_into_attributes_and_children();
            (Some(attribute), Some(child))
        } else {
            (None, None)
        }
    }
    fn fmt_attribute_or_child(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        if let Some(attributeorchild) = self {
            attributeorchild.fmt_attribute_or_child(formatter)?;
        }
        Ok(())
    }
}

pub fn html_iter<Iter: Iterator>(it: Iter) -> impl AttributeOrChild
    where
        Iter::Item: AttributeOrChild,
{
    it.collect::<Vec<_>>() // TODO this is too expensive
}
