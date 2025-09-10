(function() {
    // Game state
    let board = [
        ['', '', ''],
        ['', '', ''],
        ['', '', '']
    ];
    let currentPlayer = 'X';
    let gameOver = false;
    let winner = null;
    let scores = JSON.parse(localStorage.getItem('tictactoe-scores') || '{"X": 0, "O": 0, "draws": 0}');
    
    // Create main container
    const container = document.createElement('div');
    container.style.cssText = `
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        padding: 20px;
        max-width: 600px;
        margin: 0 auto;
        background: #fafafa;
        min-height: 100vh;
        text-align: center;
    `;
    
    // Create header
    const header = document.createElement('div');
    header.style.cssText = `
        text-align: center;
        margin-bottom: 30px;
        padding: 30px;
        background: linear-gradient(135deg, #ff6b6b 0%, #ee5a24 100%);
        color: white;
        border-radius: 12px;
        box-shadow: 0 4px 20px rgba(0,0,0,0.1);
    `;
    
    const title = document.createElement('h1');
    title.textContent = '⭕ Tic-Tac-Toe ❌';
    title.style.cssText = `
        margin: 0 0 15px 0;
        font-size: 32px;
        font-weight: 300;
        letter-spacing: -1px;
    `;
    
    const subtitle = document.createElement('h2');
    subtitle.style.cssText = `
        margin: 0;
        font-size: 18px;
        font-weight: 400;
        opacity: 0.9;
    `;
    
    // Game board container
    const gameContainer = document.createElement('div');
    gameContainer.style.cssText = `
        background: white;
        padding: 30px;
        border-radius: 12px;
        box-shadow: 0 4px 20px rgba(0,0,0,0.08);
        margin-bottom: 30px;
        border: 1px solid #e9ecef;
    `;
    
    // Canvas for the game board
    const canvas = document.createElement('canvas');
    canvas.width = 400;
    canvas.height = 400;
    canvas.style.cssText = `
        border: 3px solid #2d3748;
        border-radius: 8px;
        cursor: pointer;
        background: #f8f9fa;
        display: block;
        margin: 0 auto 20px auto;
    `;
    
    const ctx = canvas.getContext('2d');
    
    // Control buttons
    const controlsContainer = document.createElement('div');
    controlsContainer.style.cssText = `
        display: flex;
        justify-content: center;
        gap: 15px;
        margin-bottom: 20px;
    `;
    
    const newGameBtn = document.createElement('button');
    newGameBtn.textContent = '🔄 New Game';
    newGameBtn.type = 'button';
    newGameBtn.style.cssText = `
        display: inline-block;
        padding: 12px 24px;
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
    
    const resetScoresBtn = document.createElement('button');
    resetScoresBtn.textContent = '📊 Reset Scores';
    resetScoresBtn.type = 'button';
    resetScoresBtn.style.cssText = `
        display: inline-block;
        padding: 12px 24px;
        background: linear-gradient(135deg, #ffa726 0%, #ff7043 100%);
        color: white;
        border: none;
        border-radius: 8px;
        cursor: pointer;
        font-size: 16px;
        font-weight: 600;
        transition: all 0.2s ease;
        box-shadow: 0 4px 12px rgba(255, 167, 38, 0.4);
        outline: none;
    `;
    
    // Game status
    const statusContainer = document.createElement('div');
    statusContainer.style.cssText = `
        margin-bottom: 20px;
    `;
    
    const gameStatus = document.createElement('div');
    gameStatus.style.cssText = `
        font-size: 20px;
        font-weight: 600;
        color: #2d3748;
        margin-bottom: 10px;
    `;
    
    // Scoreboard
    const scoreboardContainer = document.createElement('div');
    scoreboardContainer.style.cssText = `
        background: white;
        padding: 25px;
        border-radius: 12px;
        box-shadow: 0 4px 20px rgba(0,0,0,0.08);
        border: 1px solid #e9ecef;
    `;
    
    const scoreboardTitle = document.createElement('h3');
    scoreboardTitle.textContent = '🏆 Scoreboard';
    scoreboardTitle.style.cssText = `
        margin: 0 0 20px 0;
        color: #2d3748;
        font-size: 20px;
        font-weight: 600;
    `;
    
    const scoresList = document.createElement('div');
    scoresList.style.cssText = `
        display: flex;
        justify-content: space-around;
        text-align: center;
    `;
    
    // Score items
    const playerXScore = document.createElement('div');
    playerXScore.style.cssText = `
        flex: 1;
    `;
    
    const playerOScore = document.createElement('div');
    playerOScore.style.cssText = `
        flex: 1;
    `;
    
    const drawsScore = document.createElement('div');
    drawsScore.style.cssText = `
        flex: 1;
    `;
    
    // Functions
    function updateSubtitle() {
        if (gameOver) {
            if (winner) {
                subtitle.textContent = `🎉 Player ${winner} Wins! 🎉`;
                subtitle.style.color = winner === 'X' ? '#fff' : '#fff';
            } else {
                subtitle.textContent = "🤝 It's a Draw! 🤝";
                subtitle.style.color = '#fff';
            }
        } else {
            subtitle.textContent = `Player ${currentPlayer}'s Turn`;
            subtitle.style.color = '#fff';
        }
    }
    
    function updateGameStatus() {
        if (gameOver) {
            if (winner) {
                gameStatus.textContent = `🏆 Player ${winner} is the winner!`;
                gameStatus.style.color = winner === 'X' ? '#c0392b' : '#2980b9';
            } else {
                gameStatus.textContent = '🤝 Game ended in a draw!';
                gameStatus.style.color = '#e67e22';
            }
        } else {
            gameStatus.textContent = `Current turn: Player ${currentPlayer}`;
            gameStatus.style.color = currentPlayer === 'X' ? '#c0392b' : '#2980b9';
        }
    }
    
    function updateScoreboard() {
        playerXScore.innerHTML = `
            <div style="font-size: 24px; margin-bottom: 8px;">❌</div>
            <div style="font-size: 20px; font-weight: 700; color: #e74c3c; margin-bottom: 4px;">${scores.X}</div>
            <div style="font-size: 14px; color: #6c757d;">Player X</div>
        `;
        
        playerOScore.innerHTML = `
            <div style="font-size: 24px; margin-bottom: 8px;">⭕</div>
            <div style="font-size: 20px; font-weight: 700; color: #3498db; margin-bottom: 4px;">${scores.O}</div>
            <div style="font-size: 14px; color: #6c757d;">Player O</div>
        `;
        
        drawsScore.innerHTML = `
            <div style="font-size: 24px; margin-bottom: 8px;">🤝</div>
            <div style="font-size: 20px; font-weight: 700; color: #f39c12; margin-bottom: 4px;">${scores.draws}</div>
            <div style="font-size: 14px; color: #6c757d;">Draws</div>
        `;
    }
    
    function drawBoard() {
        // Clear canvas
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        
        // Draw grid lines
        ctx.strokeStyle = '#2d3748';
        ctx.lineWidth = 4;
        ctx.lineCap = 'round';
        
        // Vertical lines
        ctx.beginPath();
        ctx.moveTo(133, 10);
        ctx.lineTo(133, 390);
        ctx.moveTo(267, 10);
        ctx.lineTo(267, 390);
        ctx.stroke();
        
        // Horizontal lines
        ctx.beginPath();
        ctx.moveTo(10, 133);
        ctx.lineTo(390, 133);
        ctx.moveTo(10, 267);
        ctx.lineTo(390, 267);
        ctx.stroke();
        
        // Draw X's and O's
        for (let row = 0; row < 3; row++) {
            for (let col = 0; col < 3; col++) {
                const x = col * 133 + 66;
                const y = row * 133 + 66;
                
                if (board[row][col] === 'X') {
                    drawX(x, y);
                } else if (board[row][col] === 'O') {
                    drawO(x, y);
                }
            }
        }
    }
    
    function drawX(x, y) {
        ctx.strokeStyle = '#e74c3c';
        ctx.lineWidth = 8;
        ctx.lineCap = 'round';
        
        ctx.beginPath();
        ctx.moveTo(x - 40, y - 40);
        ctx.lineTo(x + 40, y + 40);
        ctx.moveTo(x + 40, y - 40);
        ctx.lineTo(x - 40, y + 40);
        ctx.stroke();
    }
    
    function drawO(x, y) {
        ctx.strokeStyle = '#3498db';
        ctx.lineWidth = 8;
        ctx.lineCap = 'round';
        
        ctx.beginPath();
        ctx.arc(x, y, 40, 0, 2 * Math.PI);
        ctx.stroke();
    }
    
    function checkWinner() {
        // Check rows
        for (let row = 0; row < 3; row++) {
            if (board[row][0] && board[row][0] === board[row][1] && board[row][1] === board[row][2]) {
                return board[row][0];
            }
        }
        
        // Check columns
        for (let col = 0; col < 3; col++) {
            if (board[0][col] && board[0][col] === board[1][col] && board[1][col] === board[2][col]) {
                return board[0][col];
            }
        }
        
        // Check diagonals
        if (board[0][0] && board[0][0] === board[1][1] && board[1][1] === board[2][2]) {
            return board[0][0];
        }
        if (board[0][2] && board[0][2] === board[1][1] && board[1][1] === board[2][0]) {
            return board[0][2];
        }
        
        return null;
    }
    
    function isBoardFull() {
        for (let row = 0; row < 3; row++) {
            for (let col = 0; col < 3; col++) {
                if (board[row][col] === '') {
                    return false;
                }
            }
        }
        return true;
    }
    
    function makeMove(row, col) {
        if (gameOver || board[row][col] !== '') {
            return;
        }
        
        board[row][col] = currentPlayer;
        drawBoard();
        
        const winnerCheck = checkWinner();
        if (winnerCheck) {
            winner = winnerCheck;
            gameOver = true;
            scores[winner]++;
            localStorage.setItem('tictactoe-scores', JSON.stringify(scores));
            updateScoreboard();
            
            // Show victory animation
            setTimeout(() => {
                showNotification(`🎉 Player ${winner} wins! 🎉`, winner === 'X' ? '#e74c3c' : '#3498db');
            }, 300);
        } else if (isBoardFull()) {
            gameOver = true;
            scores.draws++;
            localStorage.setItem('tictactoe-scores', JSON.stringify(scores));
            updateScoreboard();
            
            setTimeout(() => {
                showNotification('🤝 It\'s a draw! 🤝', '#f39c12');
            }, 300);
        } else {
            currentPlayer = currentPlayer === 'X' ? 'O' : 'X';
        }
        
        updateSubtitle();
        updateGameStatus();
    }
    
    function newGame() {
        board = [
            ['', '', ''],
            ['', '', ''],
            ['', '', '']
        ];
        currentPlayer = 'X';
        gameOver = false;
        winner = null;
        
        drawBoard();
        updateSubtitle();
        updateGameStatus();
        
        showNotification('🎮 New game started!', '#667eea');
    }
    
    function resetScores() {
        scores = { X: 0, O: 0, draws: 0 };
        localStorage.setItem('tictactoe-scores', JSON.stringify(scores));
        updateScoreboard();
        showNotification('📊 Scores reset!', '#ffa726');
    }
    
    function showNotification(message, color) {
        const notification = document.createElement('div');
        notification.textContent = message;
        notification.style.cssText = `
            position: fixed;
            top: 20px;
            right: 20px;
            background: ${color};
            color: white;
            padding: 12px 20px;
            border-radius: 8px;
            font-size: 16px;
            font-weight: 600;
            z-index: 1000;
            box-shadow: 0 4px 12px rgba(0,0,0,0.3);
            transform: translateX(100%);
            transition: transform 0.3s ease;
        `;
        document.body.appendChild(notification);
        
        // Animate in
        setTimeout(() => {
            notification.style.transform = 'translateX(0)';
        }, 10);
        
        // Animate out and remove
        setTimeout(() => {
            notification.style.transform = 'translateX(100%)';
            setTimeout(() => {
                if (notification.parentNode) {
                    notification.parentNode.removeChild(notification);
                }
            }, 300);
        }, 2500);
    }
    
    // Event listeners
    canvas.onclick = function(e) {
        if (gameOver) return;
        
        const rect = canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;
        
        const col = Math.floor(x / 133);
        const row = Math.floor(y / 133);
        
        if (row >= 0 && row < 3 && col >= 0 && col < 3) {
            makeMove(row, col);
        }
    };
    
    newGameBtn.onclick = function(e) {
        e.preventDefault();
        e.stopPropagation();
        newGame();
    };
    
    newGameBtn.onmousedown = function(e) {
        e.preventDefault();
    };
    
    resetScoresBtn.onclick = function(e) {
        e.preventDefault();
        e.stopPropagation();
        resetScores();
    };
    
    resetScoresBtn.onmousedown = function(e) {
        e.preventDefault();
    };
    
    // Hover effects
    newGameBtn.onmouseenter = function() {
        this.style.transform = 'translateY(-2px)';
        this.style.boxShadow = '0 6px 20px rgba(102, 126, 234, 0.6)';
    };
    
    newGameBtn.onmouseleave = function() {
        this.style.transform = 'translateY(0)';
        this.style.boxShadow = '0 4px 12px rgba(102, 126, 234, 0.4)';
    };
    
    resetScoresBtn.onmouseenter = function() {
        this.style.transform = 'translateY(-2px)';
        this.style.boxShadow = '0 6px 20px rgba(255, 167, 38, 0.6)';
    };
    
    resetScoresBtn.onmouseleave = function() {
        this.style.transform = 'translateY(0)';
        this.style.boxShadow = '0 4px 12px rgba(255, 167, 38, 0.4)';
    };
    
    // Build DOM structure
    header.appendChild(title);
    header.appendChild(subtitle);
    
    controlsContainer.appendChild(newGameBtn);
    controlsContainer.appendChild(resetScoresBtn);
    
    statusContainer.appendChild(gameStatus);
    
    gameContainer.appendChild(canvas);
    gameContainer.appendChild(controlsContainer);
    gameContainer.appendChild(statusContainer);
    
    scoresList.appendChild(playerXScore);
    scoresList.appendChild(playerOScore);
    scoresList.appendChild(drawsScore);
    
    scoreboardContainer.appendChild(scoreboardTitle);
    scoreboardContainer.appendChild(scoresList);
    
    container.appendChild(header);
    container.appendChild(gameContainer);
    container.appendChild(scoreboardContainer);
    
    document.body.appendChild(container);
    
    // Initialize game
    drawBoard();
    updateSubtitle();
    updateGameStatus();
    updateScoreboard();
})();
