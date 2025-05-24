mod imp {
    use std::sync::OnceLock;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::clone;
    use gtk::glib;

    use adw::ButtonRow;
    use gtk::CompositeTemplate;
    use gtk::glib::subclass::Signal;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/sheet_editor_conflict_resolve_dialog.ui")]
    pub struct SheetEditorConflictResolveDialog {
        #[template_child]
        pub(super) keep_both_button: TemplateChild<ButtonRow>,
        #[template_child]
        pub(super) discard_button: TemplateChild<ButtonRow>,
        #[template_child]
        pub(super) overwrite_button: TemplateChild<ButtonRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SheetEditorConflictResolveDialog {
        const NAME: &'static str = "SheetEditorConflictResolveDialog";
        type Type = super::SheetEditorConflictResolveDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SheetEditorConflictResolveDialog {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            self.keep_both_button.get().connect_activated(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.emit_by_name::<()>("keep-both", &[]);
                    obj.close();
                }
            ));
            self.overwrite_button.get().connect_activated(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.emit_by_name::<()>("overwrite", &[]);
                    obj.close();
                }
            ));
            self.discard_button.get().connect_activated(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.emit_by_name::<()>("discard", &[]);
                    obj.close();
                }
            ));
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("keep-both").build(),
                    Signal::builder("discard").build(),
                    Signal::builder("overwrite").build(),
                ]
            })
        }
    }

    impl WidgetImpl for SheetEditorConflictResolveDialog {}
    impl AdwDialogImpl for SheetEditorConflictResolveDialog {}
}

use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct SheetEditorConflictResolveDialog(ObjectSubclass<imp::SheetEditorConflictResolveDialog>)
        @extends adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl Default for SheetEditorConflictResolveDialog {
    fn default() -> Self {
        Object::builder().build()
    }
}
