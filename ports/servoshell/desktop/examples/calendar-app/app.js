(function() {
    // Calendar state
    let currentDate = new Date();
    let events = JSON.parse(localStorage.getItem('calendar-events') || '{}');
    
    // Create main container
    const container = document.createElement('div');
    container.style.cssText = `
        font-family: Arial, sans-serif;
        padding: 20px;
        max-width: 600px;
        margin: 0 auto;
        background: #f9f9f9;
        min-height: 100vh;
    `;
    
    // Create header
    const header = document.createElement('div');
    header.style.cssText = `
        text-align: center;
        margin-bottom: 20px;
        padding: 20px;
        background: #4CAF50;
        color: white;
        border-radius: 8px;
    `;
    
    const title = document.createElement('h1');
    title.textContent = '📅 Calendar App';
    title.style.cssText = `
        margin: 0 0 10px 0;
        font-size: 24px;
    `;
    
    const monthYear = document.createElement('h2');
    monthYear.style.cssText = `
        margin: 0;
        font-size: 18px;
        font-weight: normal;
    `;
    
    // Navigation buttons
    const navContainer = document.createElement('div');
    navContainer.style.cssText = `
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-top: 15px;
    `;
    
    const prevBtn = document.createElement('button');
    prevBtn.textContent = '← Previous';
    prevBtn.style.cssText = `
        display: inline-block;
        padding: 8px 16px;
        background: #45a049;
        color: white;
        border: none;
        border-radius: 4px;
        cursor: pointer;
        font-size: 14px;
    `;
    
    const nextBtn = document.createElement('button');
    nextBtn.textContent = 'Next →';
    nextBtn.style.cssText = `
        display: inline-block;
        padding: 8px 16px;
        background: #45a049;
        color: white;
        border: none;
        border-radius: 4px;
        cursor: pointer;
        font-size: 14px;
    `;
    
    const todayBtn = document.createElement('button');
    todayBtn.textContent = 'Today';
    todayBtn.style.cssText = `
        display: inline-block;
        padding: 8px 16px;
        background: #2196F3;
        color: white;
        border: none;
        border-radius: 4px;
        cursor: pointer;
        font-size: 14px;
    `;
    
    // Calendar grid
    const calendarGrid = document.createElement('div');
    calendarGrid.style.cssText = `
        display: grid;
        grid-template-columns: repeat(7, 1fr);
        gap: 1px;
        background: #ddd;
        border-radius: 8px;
        overflow: hidden;
        margin-bottom: 20px;
    `;
    
    // Event form
    const eventForm = document.createElement('div');
    eventForm.style.cssText = `
        background: white;
        padding: 20px;
        border-radius: 8px;
        box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        margin-bottom: 20px;
    `;
    
    const formTitle = document.createElement('h3');
    formTitle.textContent = 'Add Event';
    formTitle.style.cssText = `
        margin: 0 0 15px 0;
        color: #333;
    `;
    
    const dateInput = document.createElement('input');
    dateInput.type = 'date';
    dateInput.style.cssText = `
        width: 100%;
        padding: 8px;
        margin-bottom: 10px;
        border: 1px solid #ddd;
        border-radius: 4px;
        font-size: 14px;
        box-sizing: border-box;
    `;
    
    const eventInput = document.createElement('input');
    eventInput.type = 'text';
    eventInput.placeholder = 'Enter event description...';
    eventInput.style.cssText = `
        width: 100%;
        padding: 8px;
        margin-bottom: 10px;
        border: 1px solid #ddd;
        border-radius: 4px;
        font-size: 14px;
        box-sizing: border-box;
    `;
    
    const addBtn = document.createElement('button');
    addBtn.textContent = 'Add Event';
    addBtn.style.cssText = `
        display: inline-block;
        padding: 10px 20px;
        background: #4CAF50;
        color: white;
        border: none;
        border-radius: 4px;
        cursor: pointer;
        font-size: 14px;
    `;
    
    // Events list
    const eventsContainer = document.createElement('div');
    eventsContainer.style.cssText = `
        background: white;
        border-radius: 8px;
        box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    `;
    
    const eventsTitle = document.createElement('h3');
    eventsTitle.textContent = 'Upcoming Events';
    eventsTitle.style.cssText = `
        margin: 0;
        padding: 20px 20px 10px 20px;
        color: #333;
    `;
    
    const eventsList = document.createElement('div');
    eventsList.style.cssText = `
        padding: 0 20px 20px 20px;
    `;
    
    // Functions
    function updateMonthYear() {
        const months = ['January', 'February', 'March', 'April', 'May', 'June',
                       'July', 'August', 'September', 'October', 'November', 'December'];
        monthYear.textContent = `${months[currentDate.getMonth()]} ${currentDate.getFullYear()}`;
    }
    
    function renderCalendar() {
        calendarGrid.innerHTML = '';
        
        // Day headers
        const dayHeaders = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
        dayHeaders.forEach(day => {
            const dayHeader = document.createElement('div');
            dayHeader.textContent = day;
            dayHeader.style.cssText = `
                background: #666;
                color: white;
                padding: 10px;
                text-align: center;
                font-weight: bold;
                font-size: 12px;
            `;
            calendarGrid.appendChild(dayHeader);
        });
        
        // Get first day of month and number of days
        const firstDay = new Date(currentDate.getFullYear(), currentDate.getMonth(), 1);
        const lastDay = new Date(currentDate.getFullYear(), currentDate.getMonth() + 1, 0);
        const startDate = firstDay.getDay();
        const daysInMonth = lastDay.getDate();
        
        // Empty cells for days before month starts
        for (let i = 0; i < startDate; i++) {
            const emptyDay = document.createElement('div');
            emptyDay.style.cssText = `
                background: white;
                padding: 10px;
                min-height: 40px;
            `;
            calendarGrid.appendChild(emptyDay);
        }
        
        // Days of the month
        for (let day = 1; day <= daysInMonth; day++) {
            const dayCell = document.createElement('div');
            const dateKey = `${currentDate.getFullYear()}-${String(currentDate.getMonth() + 1).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
            const hasEvent = events[dateKey] && events[dateKey].length > 0;
            const isToday = isDateToday(currentDate.getFullYear(), currentDate.getMonth(), day);
            
            dayCell.textContent = day;
            dayCell.style.cssText = `
                background: ${isToday ? '#e3f2fd' : 'white'};
                padding: 10px;
                text-align: center;
                min-height: 40px;
                cursor: pointer;
                position: relative;
                border: ${isToday ? '2px solid #2196F3' : 'none'};
                font-weight: ${isToday ? 'bold' : 'normal'};
                color: ${isToday ? '#1976d2' : '#333'};
            `;
            
            if (hasEvent) {
                const eventDot = document.createElement('div');
                eventDot.style.cssText = `
                    position: absolute;
                    bottom: 5px;
                    right: 5px;
                    width: 6px;
                    height: 6px;
                    background: #f44336;
                    border-radius: 50%;
                `;
                dayCell.appendChild(eventDot);
            }
            
            dayCell.onclick = function() {
                dateInput.value = dateKey;
                eventInput.focus();
            };
            
            calendarGrid.appendChild(dayCell);
        }
        
        updateMonthYear();
    }
    
    function isDateToday(year, month, day) {
        const today = new Date();
        return year === today.getFullYear() && 
               month === today.getMonth() && 
               day === today.getDate();
    }
    
    function renderEvents() {
        eventsList.innerHTML = '';
        
        // Get all events and sort by date
        const allEvents = [];
        for (const [date, eventArray] of Object.entries(events)) {
            eventArray.forEach((event, index) => {
                allEvents.push({ date, event, index });
            });
        }
        
        allEvents.sort((a, b) => new Date(a.date) - new Date(b.date));
        
        if (allEvents.length === 0) {
            const noEvents = document.createElement('p');
            noEvents.textContent = 'No events scheduled.';
            noEvents.style.cssText = `
                color: #666;
                font-style: italic;
                margin: 0;
            `;
            eventsList.appendChild(noEvents);
            return;
        }
        
        allEvents.forEach(({ date, event, index }) => {
            const eventItem = document.createElement('div');
            eventItem.style.cssText = `
                display: flex;
                justify-content: space-between;
                align-items: center;
                padding: 10px;
                margin-bottom: 8px;
                background: #f5f5f5;
                border-radius: 4px;
                border-left: 4px solid #4CAF50;
            `;
            
            const eventContent = document.createElement('div');
            eventContent.style.cssText = `
                flex: 1;
            `;
            
            const eventDate = document.createElement('div');
            eventDate.textContent = new Date(date).toLocaleDateString();
            eventDate.style.cssText = `
                font-size: 12px;
                color: #666;
                margin-bottom: 4px;
            `;
            
            const eventText = document.createElement('div');
            eventText.textContent = event;
            eventText.style.cssText = `
                font-size: 14px;
                color: #333;
            `;
            
            const deleteBtn = document.createElement('button');
            deleteBtn.textContent = '×';
            deleteBtn.style.cssText = `
                display: inline-block;
                background: #f44336;
                color: white;
                border: none;
                border-radius: 50%;
                width: 24px;
                height: 24px;
                cursor: pointer;
                font-size: 16px;
                line-height: 1;
            `;
            
            deleteBtn.onclick = function() {
                events[date].splice(index, 1);
                if (events[date].length === 0) {
                    delete events[date];
                }
                localStorage.setItem('calendar-events', JSON.stringify(events));
                renderEvents();
                renderCalendar();
            };
            
            eventContent.appendChild(eventDate);
            eventContent.appendChild(eventText);
            eventItem.appendChild(eventContent);
            eventItem.appendChild(deleteBtn);
            eventsList.appendChild(eventItem);
        });
    }
    
    function addEvent() {
        const date = dateInput.value;
        const eventText = eventInput.value.trim();
        
        if (!date || !eventText) {
            alert('Please enter both date and event description.');
            return;
        }
        
        if (!events[date]) {
            events[date] = [];
        }
        
        events[date].push(eventText);
        localStorage.setItem('calendar-events', JSON.stringify(events));
        
        eventInput.value = '';
        renderEvents();
        renderCalendar();
    }
    
    // Event listeners
    prevBtn.onclick = function() {
        currentDate.setMonth(currentDate.getMonth() - 1);
        renderCalendar();
    };
    
    nextBtn.onclick = function() {
        currentDate.setMonth(currentDate.getMonth() + 1);
        renderCalendar();
    };
    
    todayBtn.onclick = function() {
        currentDate = new Date();
        renderCalendar();
    };
    
    addBtn.onclick = addEvent;
    
    eventInput.onkeypress = function(e) {
        if (e.key === 'Enter') {
            addEvent();
        }
    };
    
    // Set default date to today
    dateInput.value = new Date().toISOString().split('T')[0];
    
    // Build DOM structure
    navContainer.appendChild(prevBtn);
    navContainer.appendChild(todayBtn);
    navContainer.appendChild(nextBtn);
    
    header.appendChild(title);
    header.appendChild(monthYear);
    header.appendChild(navContainer);
    
    eventForm.appendChild(formTitle);
    eventForm.appendChild(dateInput);
    eventForm.appendChild(eventInput);
    eventForm.appendChild(addBtn);
    
    eventsContainer.appendChild(eventsTitle);
    eventsContainer.appendChild(eventsList);
    
    container.appendChild(header);
    container.appendChild(calendarGrid);
    container.appendChild(eventForm);
    container.appendChild(eventsContainer);
    
    document.body.appendChild(container);
    
    // Initial render
    renderCalendar();
    renderEvents();
})();
