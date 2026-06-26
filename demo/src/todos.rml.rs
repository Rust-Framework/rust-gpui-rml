use rml::prelude::*;

pub struct TodoItem {
    pub text: String,
}

impl Default for TodoItem {
    fn default() -> Self {
        Self { text: String::new() }
    }
}

#[window(title = "RML Todos Demo", width = 400, height = 400)]
#[derive(Default)]
pub struct Todos {
    pub todos: Vec<TodoItem>,
}

impl Todos {
    #[command]
    pub fn add_todo(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.todos.push(TodoItem {
            text: format!("待办事项 #{}", self.todos.len() + 1),
        });
        cx.notify();
    }

    #[command]
    pub fn clear_todos(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.todos.clear();
        cx.notify();
    }
}
