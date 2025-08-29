// Todo List Mini-App
// A simple todo list application for the Servo Mini-App Manager

(function() {
    'use strict';
    
    // Data storage
    let todos = [];
    let nextId = 1;
    
    // Create the main container
    const container = document.createElement('div');
    container.style.cssText = `
        max-width: 500px;
        margin: 20px auto;
        padding: 20px;
        font-family: Arial, sans-serif;
        background: #ffffff;
        border: 1px solid #ddd;
    `;
    
    // Title
    const title = document.createElement('h1');
    title.textContent = 'Todo List';
    title.style.cssText = `
        color: #333;
        margin: 0 0 20px 0;
        font-size: 24px;
        text-align: center;
    `;
    
    // Input section
    const inputSection = document.createElement('div');
    inputSection.style.cssText = `
        margin-bottom: 20px;
    `;
    
    const todoInput = document.createElement('input');
    todoInput.type = 'text';
    todoInput.placeholder = 'What needs to be done?';
    todoInput.style.cssText = `
        width: 400px;
        padding: 10px;
        border: 1px solid #ccc;
        font-size: 16px;
        display: block;
        margin-bottom: 10px;
    `;
    
    const addButton = document.createElement('button');
    addButton.textContent = 'Add Todo';
    addButton.style.cssText = `
        padding: 10px 20px;
        background-color: #007AFF;
        color: white;
        border: none;
        font-size: 16px;
        display: inline-block;
        margin-top: 10px;
        cursor: pointer;
        border-radius: 4px;
    `;
    
    inputSection.appendChild(todoInput);
    inputSection.appendChild(addButton);
    
    // Todos container
    const todosContainer = document.createElement('div');
    todosContainer.style.cssText = `
        min-height: 200px;
    `;
    
    // Stats section
    const statsSection = document.createElement('div');
    statsSection.style.cssText = `
        margin-top: 20px;
        padding-top: 15px;
        border-top: 1px solid #eee;
        text-align: center;
        color: #666;
        font-size: 14px;
    `;
    
    // Add a new todo
    function addTodo() {
        const text = todoInput.value.trim();
        if (!text) return;
        
        const todo = {
            id: nextId++,
            text: text,
            completed: false
        };
        
        todos.unshift(todo);
        renderTodos();
        updateStats();
        todoInput.value = '';
    }
    
    // Toggle todo completion
    function toggleTodo(id) {
        const todo = todos.find(function(t) { return t.id === id; });
        if (todo) {
            todo.completed = !todo.completed;
            renderTodos();
            updateStats();
        }
    }
    
    // Delete a todo
    function deleteTodo(id) {
        todos = todos.filter(function(t) { return t.id !== id; });
        renderTodos();
        updateStats();
    }
    
    // Render all todos
    function renderTodos() {
        todosContainer.innerHTML = '';
        
        if (todos.length === 0) {
            const emptyMessage = document.createElement('div');
            emptyMessage.style.cssText = `
                text-align: center;
                padding: 40px 20px;
                color: #999;
                font-style: italic;
            `;
            emptyMessage.textContent = 'No tasks yet. Add one above!';
            todosContainer.appendChild(emptyMessage);
            return;
        }
        
        todos.forEach(function(todo) {
            // Create todo item container
            const todoItem = document.createElement('div');
            todoItem.style.cssText = `
                padding: 15px;
                margin-bottom: 10px;
                background: ${todo.completed ? '#f8f9fa' : 'white'};
                border: 1px solid #ddd;
            `;
            
            // Todo text
            const todoText = document.createElement('div');
            todoText.textContent = todo.text;
            todoText.style.cssText = `
                font-size: 16px;
                color: ${todo.completed ? '#666' : '#333'};
                text-decoration: ${todo.completed ? 'line-through' : 'none'};
                margin-bottom: 10px;
                display: block;
            `;
            
            // Toggle completion button
            const toggleButton = document.createElement('button');
            toggleButton.textContent = todo.completed ? '✓ Completed' : '○ Mark Complete';
            toggleButton.style.cssText = `
                background-color: ${todo.completed ? '#28a745' : '#6c757d'};
                color: white;
                border: none;
                padding: 8px 16px;
                font-size: 14px;
                display: inline-block;
                margin-right: 8px;
                cursor: pointer;
                border-radius: 4px;
            `;
            
            toggleButton.onclick = function() { 
                toggleTodo(todo.id); 
            };
            
            // Delete button
            const deleteButton = document.createElement('button');
            deleteButton.textContent = 'Delete';
            deleteButton.style.cssText = `
                background-color: #dc3545;
                color: white;
                border: none;
                padding: 8px 16px;
                font-size: 14px;
                display: inline-block;
                cursor: pointer;
                border-radius: 4px;
            `;
            
            deleteButton.onclick = function() { 
                deleteTodo(todo.id); 
            };
            
            // Assemble todo item
            todoItem.appendChild(todoText);
            todoItem.appendChild(toggleButton);
            todoItem.appendChild(deleteButton);
            todosContainer.appendChild(todoItem);
        });
    }
    
    // Update statistics
    function updateStats() {
        const total = todos.length;
        const completed = todos.filter(function(t) { return t.completed; }).length;
        const active = total - completed;
        
        statsSection.textContent = 'Total: ' + total + ' | Active: ' + active + ' | Completed: ' + completed;
    }
    
    // Event listeners
    addButton.onclick = function() {
        addTodo();
    };
    
    todoInput.onkeydown = function(e) {
        if (e.keyCode === 13) { // Enter key
            addTodo();
        }
    };
    
    // Assemble the UI
    container.appendChild(title);
    container.appendChild(inputSection);
    container.appendChild(todosContainer);
    container.appendChild(statsSection);
    
    // Set body styles
    document.body.style.cssText = `
        margin: 0;
        padding: 0;
        background: #f5f5f5;
        font-family: Arial, sans-serif;
    `;
    
    document.body.appendChild(container);
    
    // Initialize
    renderTodos();
    updateStats();
    
})();