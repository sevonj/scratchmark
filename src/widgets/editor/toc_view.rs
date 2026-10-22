mod imp {
    use crate::data::MarkdownBuffer;
    use crate::widgets::editor::toc_row::TocRow;
    use adw::glib::subclass::Signal;
    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;
    use gtk::ListBox;
    use gtk::glib;
    use gtk::glib::clone;
    use gtk::prelude::*;
    use std::cell::OnceCell;
    use std::cell::RefCell;
    use std::sync::OnceLock;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/editor/toc_view.ui")]
    pub struct TocView {
        #[template_child]
        listbox: TemplateChild<ListBox>,

        rows: RefCell<Vec<TocRow>>,

        pub(super) buffer: OnceCell<MarkdownBuffer>,
    }

    /// Table of Contents View
    #[glib::object_subclass]
    impl ObjectSubclass for TocView {
        const NAME: &'static str = "TocView";
        type Type = super::TocView;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TocView {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("activated").build()])
        }

        fn constructed(&self) {
            self.listbox.set_sort_func(move |a, b| {
                let a = a.downcast_ref::<TocRow>().unwrap();
                let b = b.downcast_ref::<TocRow>().unwrap();
                a.heading().line().cmp(&b.heading().line()).into()
            });

            self.listbox.connect_row_activated(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, row| {
                    imp.on_click(row.downcast_ref::<TocRow>().unwrap());
                }
            ));

            self.parent_constructed();
        }
    }

    impl WidgetImpl for TocView {}
    impl BinImpl for TocView {}

    impl TocView {
        pub(super) fn refresh(&self) {
            let mut rows = self.rows.borrow_mut();
            let buffer = self.buffer.get().unwrap();
            let toc = buffer.table_of_contents();

            let old_len = rows.len();
            let new_len = toc.len();

            let min_len = old_len.min(new_len);
            for i in 0..min_len {
                rows[i].update(toc[i].clone());
            }

            if new_len > old_len {
                for heading in toc.iter().skip(old_len) {
                    let row = TocRow::new(heading.clone());
                    self.listbox.append(&row);
                    rows.push(row);
                }
            } else if old_len > new_len {
                for row in rows.drain(new_len..) {
                    self.listbox.remove(&row);
                }
            }

            self.listbox.invalidate_sort();
            drop(rows);
            drop(toc);
            self.refresh_selection();
        }

        pub(super) fn refresh_selection(&self) {
            let rows = self.rows.borrow();
            let Some(buffer) = self.buffer.get() else {
                return;
            };
            let line = buffer.iter_at_offset(buffer.cursor_position()).line();
            let target_idx = rows.partition_point(|row| row.heading().line() <= line);
            if target_idx > 0 {
                self.listbox.select_row(Some(&rows[target_idx - 1]));
            } else {
                self.listbox.unselect_all();
            };
        }

        fn on_click(&self, row: &TocRow) {
            let buffer = self.buffer.get().unwrap();
            buffer.place_cursor(&buffer.iter_at_mark(row.heading().mark()));
            self.obj().emit_by_name::<()>("activated", &[]);
        }
    }
}

use crate::data::MarkdownBuffer;
use adw::glib::closure_local;
use adw::subclass::prelude::*;
use gtk::glib;
use gtk::glib::Object;
use gtk::glib::clone;
use sourceview5::prelude::*;

glib::wrapper! {
    pub struct TocView(ObjectSubclass<imp::TocView>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for TocView {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl TocView {
    pub fn bind(&self, buffer: MarkdownBuffer) {
        buffer.connect_closure(
            "toc-changed",
            true,
            closure_local!(
                #[weak(rename_to = obj)]
                self,
                move |_: MarkdownBuffer| obj.imp().refresh()
            ),
        );

        buffer.connect_cursor_moved(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| obj.imp().refresh_selection()
        ));
        self.imp().buffer.set(buffer).unwrap();
        self.imp().refresh();
    }
}
