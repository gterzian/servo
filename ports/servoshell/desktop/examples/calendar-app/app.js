(function() {
    // Calendar state
    let currentDate = new Date();
    let events = JSON.parse(localStorage.getItem('calendar-events') || '{}');
    
    // Create main container
    const container = document.createElement('div');
    container.style.cssText = `
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        padding: 20px;
        max-width: 900px;
        margin: 0 auto;
        background: #fafafa;
        min-height: 100vh;
    `;
    
    // Create header
    const header = document.createElement('div');
    header.style.cssText = `
        text-align: center;
        margin-bottom: 30px;
        padding: 30px;
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        color: white;
        border-radius: 12px;
        box-shadow: 0 4px 20px rgba(0,0,0,0.1);
    `;
    
    const title = document.createElement('h1');
    title.textContent = '📅 Calendar';
    title.style.cssText = `
        margin: 0 0 15px 0;
        font-size: 32px;
        font-weight: 300;
        letter-spacing: -1px;
    `;
    
    const monthYear = document.createElement('h2');
    monthYear.style.cssText = `
        margin: 0 0 20px 0;
        font-size: 24px;
        font-weight: 400;
        opacity: 0.95;
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
        padding: 12px 24px;
        background: rgba(255,255,255,0.2);
        color: white;
        border: 1px solid rgba(255,255,255,0.3);
        border-radius: 25px;
        cursor: pointer;
        font-size: 14px;
        font-weight: 500;
        transition: all 0.2s ease;
        backdrop-filter: blur(10px);
    `;
    
    const nextBtn = document.createElement('button');
    nextBtn.textContent = 'Next →';
    nextBtn.style.cssText = `
        display: inline-block;
        padding: 12px 24px;
        background: rgba(255,255,255,0.2);
        color: white;
        border: 1px solid rgba(255,255,255,0.3);
        border-radius: 25px;
        cursor: pointer;
        font-size: 14px;
        font-weight: 500;
        transition: all 0.2s ease;
        backdrop-filter: blur(10px);
    `;
    
    const todayBtn = document.createElement('button');
    todayBtn.textContent = 'Today';
    todayBtn.style.cssText = `
        display: inline-block;
        padding: 12px 24px;
        background: rgba(255,255,255,0.9);
        color: #667eea;
        border: none;
        border-radius: 25px;
        cursor: pointer;
        font-size: 14px;
        font-weight: 600;
        transition: all 0.2s ease;
        box-shadow: 0 2px 10px rgba(0,0,0,0.1);
    `;
    
    // Calendar grid
    const calendarGrid = document.createElement('div');
    calendarGrid.style.cssText = `
        background: white;
        border-radius: 12px;
        overflow: hidden;
        margin-bottom: 30px;
        box-shadow: 0 4px 20px rgba(0,0,0,0.08);
        border: 1px solid #e0e0e0;
    `;
    
    // Event form
    const eventForm = document.createElement('div');
    eventForm.style.cssText = `
        background: white;
        padding: 30px;
        border-radius: 12px;
        box-shadow: 0 4px 20px rgba(0,0,0,0.08);
        margin-bottom: 30px;
        border: 1px solid #e9ecef;
    `;
    
    const formTitle = document.createElement('h3');
    formTitle.textContent = 'Add New Event';
    formTitle.style.cssText = `
        margin: 0 0 20px 0;
        color: #2d3748;
        font-size: 20px;
        font-weight: 600;
    `;
    
    const dateInput = document.createElement('input');
    dateInput.type = 'date';
    dateInput.style.cssText = `
        width: 100%;
        padding: 14px 16px;
        margin-bottom: 16px;
        border: 2px solid #e2e8f0;
        border-radius: 8px;
        font-size: 16px;
        box-sizing: border-box;
        transition: border-color 0.2s ease;
        font-family: inherit;
    `;
    
    const eventInput = document.createElement('input');
    eventInput.type = 'text';
    eventInput.placeholder = 'Enter event description...';
    eventInput.style.cssText = `
        width: 100%;
        padding: 14px 16px;
        margin-bottom: 20px;
        border: 2px solid #e2e8f0;
        border-radius: 8px;
        font-size: 16px;
        box-sizing: border-box;
        transition: border-color 0.2s ease;
        font-family: inherit;
    `;
    
    const addBtn = document.createElement('button');
    addBtn.textContent = '+ Add Event';
    addBtn.type = 'button'; // Prevent form submission behavior
    addBtn.style.cssText = `
        display: inline-block;
        padding: 14px 28px;
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        color: white;
        border: none;
        border-radius: 8px;
        cursor: pointer;
        font-size: 16px;
        font-weight: 600;
        transition: all 0.2s ease;
        box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
        outline: none;
    `;
    
    // Events list
    const eventsContainer = document.createElement('div');
    eventsContainer.style.cssText = `
        background: white;
        border-radius: 12px;
        box-shadow: 0 4px 20px rgba(0,0,0,0.08);
        border: 1px solid #e9ecef;
    `;
    
    const eventsTitle = document.createElement('h3');
    eventsTitle.textContent = 'Upcoming Events';
    eventsTitle.style.cssText = `
        margin: 0;
        padding: 30px 30px 10px 30px;
        color: #2d3748;
        font-size: 20px;
        font-weight: 600;
    `;
    
    const eventsList = document.createElement('div');
    eventsList.style.cssText = `
        padding: 0 30px 30px 30px;
    `;
    
    // Functions
    function updateMonthYear() {
        const months = ['January', 'February', 'March', 'April', 'May', 'June',
                       'July', 'August', 'September', 'October', 'November', 'December'];
        monthYear.textContent = `${months[currentDate.getMonth()]} ${currentDate.getFullYear()}`;
    }
    
    function renderCalendar() {
        calendarGrid.innerHTML = '';
        
        // Create header with day names
        const headerRow = document.createElement('div');
        headerRow.style.cssText = `
            display: flex;
            background: #4a5568;
        `;
        
        const dayHeaders = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
        const dayAbbrevs = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
        dayAbbrevs.forEach((day, index) => {
            const dayHeader = document.createElement('div');
            dayHeader.textContent = day;
            dayHeader.title = dayHeaders[index];
            dayHeader.style.cssText = `
                flex: 1;
                color: white;
                padding: 16px 8px;
                text-align: center;
                font-weight: 600;
                font-size: 13px;
                letter-spacing: 0.5px;
                text-transform: uppercase;
                border-right: 1px solid rgba(255,255,255,0.1);
            `;
            if (index === 6) { // Remove border from last column
                dayHeader.style.borderRight = 'none';
            }
            headerRow.appendChild(dayHeader);
        });
        calendarGrid.appendChild(headerRow);
        
        // Create calendar body
        const calendarBody = document.createElement('div');
        calendarBody.style.cssText = `
            display: flex;
            flex-direction: column;
        `;
        
        // Get first day of month and number of days
        const firstDay = new Date(currentDate.getFullYear(), currentDate.getMonth(), 1);
        const lastDay = new Date(currentDate.getFullYear(), currentDate.getMonth() + 1, 0);
        const startDate = firstDay.getDay();
        const daysInMonth = lastDay.getDate();
        
        // Get previous month's last days for empty cells
        const prevMonth = new Date(currentDate.getFullYear(), currentDate.getMonth(), 0);
        const prevMonthDays = prevMonth.getDate();
        
        // Calculate how many weeks we need
        const totalDays = startDate + daysInMonth;
        const weeksNeeded = Math.ceil(totalDays / 7);
        
        let dayCounter = 1;
        let nextMonthDay = 1;
        
        // Create weeks
        for (let week = 0; week < weeksNeeded; week++) {
            const weekRow = document.createElement('div');
            weekRow.style.cssText = `
                display: flex;
                border-bottom: 1px solid #e0e0e0;
            `;
            if (week === weeksNeeded - 1) { // Remove border from last row
                weekRow.style.borderBottom = 'none';
            }
            
            // Create 7 days for this week
            for (let dayOfWeek = 0; dayOfWeek < 7; dayOfWeek++) {
                const dayCell = document.createElement('div');
                const totalDayIndex = week * 7 + dayOfWeek;
                
                dayCell.style.cssText = `
                    flex: 1;
                    min-height: 120px;
                    padding: 12px 8px 8px 8px;
                    border-right: 1px solid #e0e0e0;
                    cursor: pointer;
                    position: relative;
                    overflow: hidden;
                    display: flex;
                    flex-direction: column;
                `;
                if (dayOfWeek === 6) { // Remove border from last column
                    dayCell.style.borderRight = 'none';
                }
                
                let dayNumber, dateKey, isCurrentMonth = false, isToday = false, isWeekend = false;
                
                if (totalDayIndex < startDate) {
                    // Previous month days
                    dayNumber = prevMonthDays - (startDate - totalDayIndex - 1);
                    dayCell.style.background = '#f8f9fa';
                    dayCell.style.color = '#adb5bd';
                } else if (dayCounter <= daysInMonth) {
                    // Current month days
                    dayNumber = dayCounter;
                    dateKey = `${currentDate.getFullYear()}-${String(currentDate.getMonth() + 1).padStart(2, '0')}-${String(dayNumber).padStart(2, '0')}`;
                    isCurrentMonth = true;
                    isToday = isDateToday(currentDate.getFullYear(), currentDate.getMonth(), dayNumber);
                    isWeekend = dayOfWeek === 0 || dayOfWeek === 6;
                    
                    if (isToday) {
                        dayCell.style.background = '#e3f2fd';
                        dayCell.style.border = '2px solid #1976d2';
                        dayCell.style.borderRight = dayOfWeek === 6 ? '2px solid #1976d2' : '2px solid #1976d2';
                    } else if (isWeekend) {
                        dayCell.style.background = '#fafafa';
                    } else {
                        dayCell.style.background = 'white';
                    }
                    
                    dayCounter++;
                } else {
                    // Next month days
                    dayNumber = nextMonthDay;
                    dayCell.style.background = '#f8f9fa';
                    dayCell.style.color = '#adb5bd';
                    nextMonthDay++;
                }
                
                // Day number
                const dayNumberElement = document.createElement('div');
                dayNumberElement.textContent = dayNumber;
                dayNumberElement.style.cssText = `
                    font-size: 16px;
                    font-weight: ${isToday ? '700' : '600'};
                    color: ${isToday ? '#1976d2' : (isCurrentMonth ? (isWeekend ? '#6c757d' : '#212529') : '#adb5bd')};
                    margin-bottom: 8px;
                    align-self: flex-start;
                `;
                dayCell.appendChild(dayNumberElement);
                
                // Events container for current month days
                if (isCurrentMonth && dateKey) {
                    const dayEvents = events[dateKey] || [];
                    const eventsContainer = document.createElement('div');
                    eventsContainer.style.cssText = `
                        display: flex;
                        flex-direction: column;
                        gap: 2px;
                        flex: 1;
                        overflow: hidden;
                    `;
                    
                    // Show up to 3 events directly in the cell
                    dayEvents.slice(0, 3).forEach((event) => {
                        const eventElement = document.createElement('div');
                        eventElement.textContent = event.length > 12 ? event.substring(0, 12) + '...' : event;
                        eventElement.title = event; // Full text on hover
                        eventElement.style.cssText = `
                            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                            color: white;
                            padding: 2px 6px;
                            border-radius: 10px;
                            font-size: 10px;
                            font-weight: 500;
                            text-overflow: ellipsis;
                            white-space: nowrap;
                            overflow: hidden;
                            opacity: 0.9;
                        `;
                        eventsContainer.appendChild(eventElement);
                    });
                    
                    // Show "more" indicator if there are more than 3 events
                    if (dayEvents.length > 3) {
                        const moreElement = document.createElement('div');
                        moreElement.textContent = `+${dayEvents.length - 3} more`;
                        moreElement.style.cssText = `
                            color: #6c757d;
                            font-size: 9px;
                            font-weight: 500;
                            text-align: center;
                            margin-top: 2px;
                        `;
                        eventsContainer.appendChild(moreElement);
                    }
                    
                    dayCell.appendChild(eventsContainer);
                    
                    // Click handler for current month days only
                    dayCell.onclick = function() {
                        dateInput.value = dateKey;
                        eventInput.focus();
                        // Scroll to form
                        eventForm.scrollIntoView({ behavior: 'smooth', block: 'center' });
                    };
                }
                
                // Hover effect for current month days
                if (isCurrentMonth) {
                    dayCell.onmouseenter = function() {
                        if (!isToday) {
                            this.style.background = '#f0f7ff';
                        }
                        this.style.transform = 'scale(1.02)';
                        this.style.boxShadow = '0 4px 12px rgba(0,0,0,0.1)';
                        this.style.zIndex = '10';
                    };
                    
                    dayCell.onmouseleave = function() {
                        if (!isToday) {
                            this.style.background = isWeekend ? '#fafafa' : 'white';
                        }
                        this.style.transform = 'scale(1)';
                        this.style.boxShadow = 'none';
                        this.style.zIndex = '1';
                    };
                }
                
                weekRow.appendChild(dayCell);
            }
            
            calendarBody.appendChild(weekRow);
        }
        
        calendarGrid.appendChild(calendarBody);
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
            const noEvents = document.createElement('div');
            noEvents.innerHTML = `
                <div style="text-align: center; padding: 40px 20px; color: #6c757d;">
                    <div style="font-size: 48px; margin-bottom: 16px; opacity: 0.5;">📅</div>
                    <div style="font-size: 16px; font-weight: 500; margin-bottom: 8px;">No events scheduled</div>
                    <div style="font-size: 14px; opacity: 0.8;">Click on a day to add your first event</div>
                </div>
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
                padding: 16px 20px;
                margin-bottom: 12px;
                background: linear-gradient(135deg, #f8f9ff 0%, #f0f4ff 100%);
                border-radius: 8px;
                border-left: 4px solid #667eea;
                transition: all 0.2s ease;
                cursor: pointer;
            `;
            
            const eventContent = document.createElement('div');
            eventContent.style.cssText = `
                flex: 1;
            `;
            
            const eventDate = document.createElement('div');
            const dateObj = new Date(date);
            const options = { 
                weekday: 'short', 
                year: 'numeric', 
                month: 'short', 
                day: 'numeric' 
            };
            eventDate.textContent = dateObj.toLocaleDateString('en-US', options);
            eventDate.style.cssText = `
                font-size: 12px;
                color: #667eea;
                margin-bottom: 6px;
                font-weight: 600;
                text-transform: uppercase;
                letter-spacing: 0.5px;
            `;
            
            const eventText = document.createElement('div');
            eventText.textContent = event;
            eventText.style.cssText = `
                font-size: 16px;
                color: #2d3748;
                font-weight: 500;
                line-height: 1.4;
            `;
            
            const deleteBtn = document.createElement('button');
            deleteBtn.innerHTML = '×';
            deleteBtn.type = 'button'; // Prevent form behavior
            deleteBtn.style.cssText = `
                display: inline-block;
                background: #e53e3e;
                color: white;
                border: none;
                border-radius: 50%;
                width: 28px;
                height: 28px;
                cursor: pointer;
                font-size: 18px;
                line-height: 1;
                transition: all 0.2s ease;
                box-shadow: 0 2px 8px rgba(229, 62, 62, 0.3);
                outline: none;
            `;
            
            // Prevent focus issues
            deleteBtn.onmousedown = function(e) {
                e.preventDefault();
            };
            
            // Hover effects
            eventItem.onmouseenter = function() {
                this.style.transform = 'translateY(-2px)';
                this.style.boxShadow = '0 8px 25px rgba(0,0,0,0.1)';
            };
            
            eventItem.onmouseleave = function() {
                this.style.transform = 'translateY(0)';
                this.style.boxShadow = 'none';
            };
            
            deleteBtn.onmouseenter = function() {
                this.style.background = '#c53030';
                this.style.transform = 'scale(1.1)';
            };
            
            deleteBtn.onmouseleave = function() {
                this.style.background = '#e53e3e';
                this.style.transform = 'scale(1)';
            };
            
            deleteBtn.onclick = function(e) {
                e.preventDefault();
                e.stopPropagation();
                
                // Create custom confirmation dialog instead of confirm()
                const confirmDialog = document.createElement('div');
                confirmDialog.style.cssText = `
                    position: fixed;
                    top: 0;
                    left: 0;
                    right: 0;
                    bottom: 0;
                    background: rgba(0,0,0,0.5);
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    z-index: 2000;
                `;
                
                const dialogContent = document.createElement('div');
                dialogContent.style.cssText = `
                    background: white;
                    padding: 30px;
                    border-radius: 12px;
                    box-shadow: 0 10px 30px rgba(0,0,0,0.3);
                    max-width: 400px;
                    text-align: center;
                `;
                
                const dialogText = document.createElement('p');
                dialogText.textContent = 'Are you sure you want to delete this event?';
                dialogText.style.cssText = `
                    margin: 0 0 20px 0;
                    font-size: 16px;
                    color: #333;
                `;
                
                const buttonContainer = document.createElement('div');
                buttonContainer.style.cssText = `
                    display: flex;
                    gap: 12px;
                    justify-content: center;
                `;
                
                const cancelBtn = document.createElement('button');
                cancelBtn.textContent = 'Cancel';
                cancelBtn.style.cssText = `
                    padding: 10px 20px;
                    background: #6c757d;
                    color: white;
                    border: none;
                    border-radius: 6px;
                    cursor: pointer;
                    font-size: 14px;
                `;
                
                const confirmBtn = document.createElement('button');
                confirmBtn.textContent = 'Delete';
                confirmBtn.style.cssText = `
                    padding: 10px 20px;
                    background: #e53e3e;
                    color: white;
                    border: none;
                    border-radius: 6px;
                    cursor: pointer;
                    font-size: 14px;
                `;
                
                cancelBtn.onclick = function() {
                    document.body.removeChild(confirmDialog);
                };
                
                confirmBtn.onclick = function() {
                    document.body.removeChild(confirmDialog);
                    events[date].splice(index, 1);
                    if (events[date].length === 0) {
                        delete events[date];
                    }
                    localStorage.setItem('calendar-events', JSON.stringify(events));
                    renderEvents();
                    renderCalendar();
                    
                    // Show success notification
                    const notification = document.createElement('div');
                    notification.textContent = 'Event deleted successfully!';
                    notification.style.cssText = `
                        position: fixed;
                        top: 20px;
                        right: 20px;
                        background: #e53e3e;
                        color: white;
                        padding: 12px 20px;
                        border-radius: 8px;
                        font-size: 14px;
                        z-index: 1000;
                        box-shadow: 0 4px 12px rgba(0,0,0,0.3);
                    `;
                    document.body.appendChild(notification);
                    setTimeout(() => {
                        if (notification.parentNode) {
                            notification.parentNode.removeChild(notification);
                        }
                    }, 2000);
                };
                
                buttonContainer.appendChild(cancelBtn);
                buttonContainer.appendChild(confirmBtn);
                dialogContent.appendChild(dialogText);
                dialogContent.appendChild(buttonContainer);
                confirmDialog.appendChild(dialogContent);
                document.body.appendChild(confirmDialog);
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
            // Create a simple notification instead of alert
            const notification = document.createElement('div');
            notification.textContent = 'Please enter both date and event description.';
            notification.style.cssText = `
                position: fixed;
                top: 20px;
                right: 20px;
                background: #f44336;
                color: white;
                padding: 12px 20px;
                border-radius: 8px;
                font-size: 14px;
                z-index: 1000;
                box-shadow: 0 4px 12px rgba(0,0,0,0.3);
            `;
            document.body.appendChild(notification);
            setTimeout(() => {
                if (notification.parentNode) {
                    notification.parentNode.removeChild(notification);
                }
            }, 3000);
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
        
        // Show success notification
        const notification = document.createElement('div');
        notification.textContent = 'Event added successfully!';
        notification.style.cssText = `
            position: fixed;
            top: 20px;
            right: 20px;
            background: #4CAF50;
            color: white;
            padding: 12px 20px;
            border-radius: 8px;
            font-size: 14px;
            z-index: 1000;
            box-shadow: 0 4px 12px rgba(0,0,0,0.3);
        `;
        document.body.appendChild(notification);
        setTimeout(() => {
            if (notification.parentNode) {
                notification.parentNode.removeChild(notification);
            }
        }, 2000);
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
    
    addBtn.onclick = function(e) {
        e.preventDefault();
        e.stopPropagation();
        addEvent();
    };
    
    // Also handle mousedown to prevent focus issues
    addBtn.onmousedown = function(e) {
        e.preventDefault();
    };
    
    // Add hover effects for buttons
    addBtn.onmouseenter = function() {
        this.style.transform = 'translateY(-2px)';
        this.style.boxShadow = '0 6px 20px rgba(102, 126, 234, 0.6)';
    };
    
    addBtn.onmouseleave = function() {
        this.style.transform = 'translateY(0)';
        this.style.boxShadow = '0 4px 12px rgba(102, 126, 234, 0.4)';
    };
    
    // Add focus effects for inputs
    dateInput.onfocus = function() {
        this.style.borderColor = '#667eea';
        this.style.boxShadow = '0 0 0 3px rgba(102, 126, 234, 0.1)';
    };
    
    dateInput.onblur = function() {
        this.style.borderColor = '#e2e8f0';
        this.style.boxShadow = 'none';
    };
    
    eventInput.onfocus = function() {
        this.style.borderColor = '#667eea';
        this.style.boxShadow = '0 0 0 3px rgba(102, 126, 234, 0.1)';
    };
    
    eventInput.onblur = function() {
        this.style.borderColor = '#e2e8f0';
        this.style.boxShadow = 'none';
    };
    
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
