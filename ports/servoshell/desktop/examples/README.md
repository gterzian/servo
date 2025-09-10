# Mini-App Development Guide

This guide explains how to create mini-apps for the Servo super-app platform.

## Directory Structure

Each mini-app must be placed in its own directory within the `examples` folder. The directory name will be used as the app identifier.

```
examples/
├── my-app/
│   ├── manifest.json
│   └── app.js
```

## Required Files

### manifest.json

The manifest file contains metadata about your mini-app. It must include the following fields:

```json
{
  "title": "My App",
  "application": "app.js",
  "description": "A description of what the app does",
  "version": "1.0.0"
}
```

#### Fields

- `title` (string): The display name shown in the app tabs
- `application` (string): The JavaScript file to execute (typically "app.js")
- `description` (string): A brief description of the app's functionality
- `version` (string): Version number in semantic versioning format

### app.js

The main JavaScript file that contains your app's code. This file should:

- Be wrapped in an immediately invoked function expression (IIFE)
- Create and manage its own DOM elements
- Handle all user interactions
- Use localStorage for data persistence if needed

## JavaScript Structure

Your app.js should follow this basic structure:

```javascript
(function() {
    // App state and variables
    
    // Create DOM elements
    const container = document.createElement('div');
    // ... build your UI
    
    // Event handlers and functions
    function myFunction() {
        // App logic
    }
    
    // Attach to document
    document.body.appendChild(container);
    
    // Initialize the app
    // ... startup code
})();
```

## Best Practices

### DOM Management
- Always create a root container element for your app
- Use proper CSS styling with the `style.cssText` property
- Clean up event listeners if your app is unloaded

### Event Handling
- Use `e.preventDefault()` and `e.stopPropagation()` on button clicks
- Add `onmousedown` handlers with `preventDefault()` to avoid focus issues
- Set button `type="button"` to prevent form submission behavior

### Styling
- Use inline styles via `element.style.cssText` for compatibility
- Follow consistent color schemes and spacing
- Make buttons and interactive elements clearly visible

## Example Apps

The examples directory contains several reference implementations:

- `calendar-app`: Event scheduling with date picker and persistence
- `notes-app`: Simple text note taking and management
- `todo-list`: Task management with completion tracking
- `tic-tac-toe`: Canvas-based game with score tracking
