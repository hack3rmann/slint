use smol_str::SmolStr;

use crate::{
    diagnostics::BuildDiagnostics,
    langtype::ElementType,
    namedreference::NamedReference,
    object_tree::{self, ChildProcess, Component, ElementRc},
};
use std::rc::Rc;

pub fn lower_child_processes(component: &Rc<Component>, diag: &mut BuildDiagnostics) {
    object_tree::recurse_elem_including_sub_components_no_borrow(
        component,
        &None,
        &mut |elem, parent_element: &Option<ElementRc>| {
            let is_child_process = matches!(&elem.borrow().base_type, ElementType::Builtin(base_type) if base_type.name == "ChildProcess");

            if is_child_process {
                lower_child_process(elem, parent_element.as_ref(), diag);
            }

            Some(elem.clone())
        },
    );
}

fn lower_child_process(
    elem: &ElementRc,
    parent_elem: Option<&ElementRc>,
    diag: &mut BuildDiagnostics,
) {
    let parent_component = elem.borrow().enclosing_component.upgrade().unwrap();
    let Some(parent_elem) = parent_elem else {
        diag.push_error("A component cannot inherit from ChildProcess".into(), &*elem.borrow());
        return;
    };

    if Rc::ptr_eq(&parent_component.root_element, elem) {
        diag.push_error(
            "ChildProcess cannot be directly repeated or conditional".into(),
            &*elem.borrow(),
        );
        return;
    }

    if !elem.borrow().is_binding_set("command", true) {
        diag.push_error(
            "ChildProcess must have a binding set for its 'command' property".into(),
            &*elem.borrow(),
        );
        return;
    }

    // Remove the child_process_element from its parent
    let index = {
        let mut parent_element_borrowed = parent_elem.borrow_mut();
        let index = parent_element_borrowed
            .children
            .iter()
            .position(|child| Rc::ptr_eq(child, elem))
            .expect("ChildProcess must be a child of its parent");

        let removed = parent_element_borrowed.children.remove(index);
        parent_component.optimized_elements.borrow_mut().push(removed);

        index
    };

    if let Some(parent_cip) = &mut *parent_component.child_insertion_point.borrow_mut()
        && Rc::ptr_eq(&parent_cip.parent, parent_elem)
        && parent_cip.insertion_index > index
    {
        parent_cip.insertion_index -= 1;
    }

    let command = NamedReference::new(elem, SmolStr::new("command"));
    command.mark_as_set();
    let stdout_line = NamedReference::new(elem, SmolStr::new("stdout-line"));
    stdout_line.mark_as_set();
    let stderr_line = NamedReference::new(elem, SmolStr::new("stderr-line"));
    stderr_line.mark_as_set();

    parent_component.child_processes.borrow_mut().push(ChildProcess {
        command,
        stdout_line,
        stderr_line,
        element: Rc::downgrade(elem),
    });
}
