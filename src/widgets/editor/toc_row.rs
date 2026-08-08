mod imp {
    use crate::data::MarkdownHeading;
    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;
    use gtk::Label;
    use gtk::ListBoxRow;
    use gtk::TemplateChild;
    use gtk::glib;
    use gtk::glib::Properties;
    use std::cell::OnceCell;

    #[derive(CompositeTemplate, Default, Properties)]
    #[properties(wrapper_type = super::TocRow)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/editor/toc_row.ui")]
    pub struct TocRow {
        #[template_child]
        pub(super) label: TemplateChild<Label>,
        #[template_child]
        pub(super) line: TemplateChild<Label>,

        pub(super) heading: OnceCell<MarkdownHeading>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TocRow {
        const NAME: &'static str = "TocRow";
        type Type = super::TocRow;
        type ParentType = ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for TocRow {}

    impl WidgetImpl for TocRow {}
    impl ListBoxRowImpl for TocRow {}

    impl TocRow {
        pub(super) fn bind(&self, heading: MarkdownHeading) {
            self.label.set_text(heading.text());
            self.line.set_text(&(heading.line() + 1).to_string());
            self.heading.set(heading).unwrap();
        }
    }
}

use crate::data::MarkdownHeading;
use adw::subclass::prelude::*;
use gtk::ListBoxRow;
use gtk::glib;
use gtk::glib::Object;

glib::wrapper! {
    pub struct TocRow(ObjectSubclass<imp::TocRow>)
        @extends ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl TocRow {
    pub fn new(heading: MarkdownHeading) -> Self {
        let obj: Self = Object::builder().build();
        let imp = obj.imp();
        imp.bind(heading);
        obj
    }

    pub fn heading(&self) -> &MarkdownHeading {
        self.imp().heading.get().unwrap()
    }
}
