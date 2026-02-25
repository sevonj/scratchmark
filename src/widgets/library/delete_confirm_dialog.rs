mod imp {
    use adw::AlertDialog;
    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;
    use gtk::glib;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/library/delete_confirm_dialog.ui")]
    pub struct DeleteConfirmDialog {}

    #[glib::object_subclass]
    impl ObjectSubclass for DeleteConfirmDialog {
        const NAME: &'static str = "DeleteConfirmDialog";
        type Type = super::DeleteConfirmDialog;
        type ParentType = AlertDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DeleteConfirmDialog {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for DeleteConfirmDialog {}
    impl AdwDialogImpl for DeleteConfirmDialog {}
    impl AdwAlertDialogImpl for DeleteConfirmDialog {}
}

use adw::AlertDialog;
use adw::prelude::*;
use gtk::glib;
use gtk::glib::Object;

glib::wrapper! {
    pub struct DeleteConfirmDialog(ObjectSubclass<imp::DeleteConfirmDialog>)
        @extends adw::AlertDialog, adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl DeleteConfirmDialog {
    pub fn new(filename: &str) -> Self {
        let obj: DeleteConfirmDialog = Object::builder().build();
        let body = obj.body();
        obj.set_body(&body.replace("{{filename}}", filename));
        obj
    }

    pub fn present(&self, parent: Option<&impl glib::object::IsA<gtk::Widget>>) {
        self.clone().upcast::<AlertDialog>().present(parent);
    }
}
