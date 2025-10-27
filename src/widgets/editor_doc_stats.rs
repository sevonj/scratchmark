mod imp {
    use adw::subclass::prelude::*;
    use gtk::glib;

    use gtk::CompositeTemplate;
    use gtk::Label;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/editor_doc_stats.ui")]
    pub struct EditorDocStats {
        #[template_child]
        pub(super) lab_num_chars: TemplateChild<Label>,
        #[template_child]
        pub(super) lab_num_nospace: TemplateChild<Label>,
        #[template_child]
        pub(super) lab_num_words: TemplateChild<Label>,
        #[template_child]
        pub(super) lab_num_lines: TemplateChild<Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EditorDocStats {
        const NAME: &'static str = "EditorDocStats";
        type Type = super::EditorDocStats;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EditorDocStats {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for EditorDocStats {}
    impl BinImpl for EditorDocStats {}
}

use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::glib;

use crate::widgets::editor::DocumentStatsData;

glib::wrapper! {
    pub struct EditorDocStats(ObjectSubclass<imp::EditorDocStats>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for EditorDocStats {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl EditorDocStats {
    pub fn set_stats(&self, data: &DocumentStatsData) {
        let imp = self.imp();
        imp.lab_num_chars.set_label(&format!("{}", data.num_chars));
        imp.lab_num_nospace
            .set_label(&format!("{}", data.num_chars - data.num_spaces));
        imp.lab_num_words.set_label(&format!("{}", data.num_words));
        imp.lab_num_lines.set_label(&format!("{}", data.num_lines));
    }
}
