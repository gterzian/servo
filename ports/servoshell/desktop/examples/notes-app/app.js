// Notes App - Simple note-taking with reverse chronological order
// Following the same pattern as todo-list example for Servo compatibility

(function() {
    'use strict';
    
    // Data storage
    let notes = [];
    let nextId = 1;
    
    // Load notes from localStorage on startup
    loadNotes();
    
    // Create the main container
    const container = document.createElement('div');
    container.style.cssText = `
        max-width: 600px;
        margin: 20px auto;
        padding: 20px;
        font-family: Arial, sans-serif;
        background: #ffffff;
        border: 1px solid #ddd;
        border-radius: 8px;
        box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    `;
    
    // Title
    const title = document.createElement('h1');
    title.textContent = '📝 Notes App';
    title.style.cssText = `
        color: #333;
        margin: 0 0 30px 0;
        font-size: 28px;
        text-align: center;
    `;
    
    // Input section
    const inputSection = document.createElement('div');
    inputSection.style.cssText = `
        margin-bottom: 30px;
        padding: 20px;
        background-color: #f9f9f9;
        border-radius: 6px;
        border: 1px solid #ddd;
    `;
    
    const inputLabel = document.createElement('label');
    inputLabel.textContent = 'Add a new note:';
    inputLabel.style.cssText = `
        display: block;
        margin-bottom: 5px;
        font-weight: bold;
        color: #555;
    `;
    
    const noteTextarea = document.createElement('textarea');
    noteTextarea.placeholder = 'Type your note here...';
    noteTextarea.style.cssText = `
        display: block;
        width: 100%;
        min-height: 80px;
        padding: 10px;
        border: 1px solid #ccc;
        border-radius: 4px;
        font-size: 14px;
        font-family: Arial, sans-serif;
        resize: vertical;
        box-sizing: border-box;
        margin-bottom: 15px;
    `;
    
    const addButton = document.createElement('button');
    addButton.textContent = 'Add Note';
    addButton.style.cssText = `
        display: inline-block;
        background-color: #007bff;
        color: white;
        border: none;
        padding: 10px 20px;
        border-radius: 4px;
        font-size: 16px;
        cursor: pointer;
    `;
    
    inputSection.appendChild(inputLabel);
    inputSection.appendChild(noteTextarea);
    inputSection.appendChild(addButton);
    
    // Notes container
    const notesContainer = document.createElement('div');
    notesContainer.style.cssText = `
        min-height: 200px;
    `;
    
    // Clear all button
    const clearButton = document.createElement('button');
    clearButton.textContent = 'Clear All Notes';
    clearButton.style.cssText = `
        display: none;
        background-color: #dc3545;
        color: white;
        border: none;
        padding: 8px 16px;
        border-radius: 4px;
        font-size: 14px;
        cursor: pointer;
        margin-top: 20px;
    `;
    
    // Add a new note
    function addNote() {
        const text = noteTextarea.value.trim();
        if (!text) return;
        
        const note = {
            id: nextId++,
            content: text,
            timestamp: new Date().toLocaleString()
        };
        
        // Add to beginning for reverse chronological order
        notes.unshift(note);
        saveNotes();
        renderNotes();
        noteTextarea.value = '';
    }
    
    // Delete a note
    function deleteNote(id) {
        notes = notes.filter(function(n) { return n.id !== id; });
        saveNotes();
        renderNotes();
    }
    
    // Clear all notes
    function clearAllNotes() {
        if (notes.length === 0) return;
        
        // Simple confirmation using confirm()
        if (confirm('Are you sure you want to delete all notes?')) {
            notes = [];
            saveNotes();
            renderNotes();
        }
    }
    
    // Render all notes
    function renderNotes() {
        notesContainer.innerHTML = '';
        
        if (notes.length === 0) {
            const emptyMessage = document.createElement('div');
            emptyMessage.style.cssText = `
                text-align: center;
                color: #666;
                font-style: italic;
                padding: 40px 20px;
            `;
            emptyMessage.textContent = 'No notes yet. Add your first note above!';
            notesContainer.appendChild(emptyMessage);
            clearButton.style.display = 'none';
            return;
        }
        
        // Show notes in reverse chronological order
        notes.forEach(function(note) {
            // Create note item container
            const noteItem = document.createElement('div');
            noteItem.style.cssText = `
                display: block;
                background-color: white;
                border: 1px solid #e0e0e0;
                border-radius: 6px;
                padding: 15px;
                margin-bottom: 15px;
                box-shadow: 0 1px 3px rgba(0,0,0,0.1);
            `;
            
            // Note content
            const noteContent = document.createElement('div');
            noteContent.textContent = note.content;
            noteContent.style.cssText = `
                display: block;
                color: #333;
                line-height: 1.5;
                margin-bottom: 10px;
                white-space: pre-wrap;
                word-wrap: break-word;
            `;
            
            // Timestamp and delete button container
            const timestampContainer = document.createElement('div');
            timestampContainer.style.cssText = `
                display: block;
                color: #666;
                font-size: 12px;
                text-align: right;
            `;
            
            // Timestamp
            const timestamp = document.createElement('span');
            timestamp.textContent = note.timestamp;
            timestamp.style.cssText = `
                display: inline-block;
                margin-right: 10px;
            `;
            
            // Delete button
            const deleteButton = document.createElement('button');
            deleteButton.textContent = 'Delete';
            deleteButton.style.cssText = `
                display: inline-block;
                background-color: #dc3545;
                color: white;
                border: none;
                padding: 4px 8px;
                border-radius: 3px;
                font-size: 12px;
                cursor: pointer;
            `;
            
            deleteButton.onclick = function() {
                deleteNote(note.id);
            };
            
            timestampContainer.appendChild(timestamp);
            timestampContainer.appendChild(deleteButton);
            
            noteItem.appendChild(noteContent);
            noteItem.appendChild(timestampContainer);
            notesContainer.appendChild(noteItem);
        });
        
        clearButton.style.display = 'block';
    }
    
    // Save notes to localStorage
    function saveNotes() {
        try {
            localStorage.setItem('notesApp_notes', JSON.stringify(notes));
        } catch (e) {
            console.error('Failed to save notes:', e);
        }
    }
    
    // Load notes from localStorage
    function loadNotes() {
        try {
            const saved = localStorage.getItem('notesApp_notes');
            if (saved) {
                const loadedNotes = JSON.parse(saved);
                notes = loadedNotes;
                // Find the highest ID to continue sequence
                if (notes.length > 0) {
                    nextId = Math.max.apply(Math, notes.map(function(n) { return n.id; })) + 1;
                }
            }
        } catch (e) {
            console.error('Failed to load notes:', e);
            notes = [];
        }
    }
    
    // Event listeners
    addButton.onclick = function() {
        addNote();
    };
    
    clearButton.onclick = function() {
        clearAllNotes();
    };
    
    // Allow Enter key to add note (with Ctrl/Cmd for new line)
    noteTextarea.onkeydown = function(e) {
        if (e.keyCode === 13 && !e.ctrlKey && !e.metaKey) { // Enter without modifiers
            e.preventDefault();
            addNote();
        }
    };
    
    // Assemble the UI
    container.appendChild(title);
    container.appendChild(inputSection);
    container.appendChild(notesContainer);
    container.appendChild(clearButton);
    
    // Set body styles
    document.body.style.cssText = `
        margin: 0;
        padding: 0;
        background: #f5f5f5;
        font-family: Arial, sans-serif;
    `;
    
    document.body.appendChild(container);
    
    // Initialize
    renderNotes();
    
    // Focus the textarea
    noteTextarea.focus();
    
})();
