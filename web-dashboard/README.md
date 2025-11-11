# 📊 BitQuan Testnet Web Dashboard

Beautiful, real-time web dashboard for BitQuan testnet monitoring.

## 🎯 Features

- ✅ **Real-time Stats** - Block height, hashrate, difficulty
- ✅ **Recent Blocks** - Latest mined blocks with details
- ✅ **Network Info** - Active nodes, transaction count
- ✅ **Responsive Design** - Works on desktop & mobile
- ✅ **Beautiful UI** - Glassmorphism design with animations

## 🚀 Quick Start

### Option 1: Python Server (Recommended)
```bash
cd web-dashboard
python3 server.py
```

Then open: **http://localhost:8080**

### Option 2: Direct File
```bash
cd web-dashboard/public
open index.html
```

### Option 3: Any HTTP Server
```bash
cd web-dashboard/public

# Using Python
python3 -m http.server 8080

# Using Node.js
npx http-server -p 8080

# Using PHP
php -S localhost:8080
```

## 📸 Screenshot

The dashboard shows:
- Block Height
- Network Hashrate  
- Mining Difficulty
- Active Nodes
- Transaction Count
- Block Time
- Recent Blocks List

## 🔧 Configuration

### Update RPC Endpoint

Edit `public/index.html` and change the API endpoint:

```javascript
const RPC_ENDPOINT = 'http://localhost:8334';
// or
const RPC_ENDPOINT = 'https://claims-upcoming-cho-vid.trycloudflare.com';
```

### Enable Real Data

Replace mock data with real API calls:

```javascript
async function fetchBlockchainInfo() {
    try {
        const response = await fetch('http://localhost:8334/api/info');
        const data = await response.json();
        
        document.getElementById('blockHeight').textContent = data.height;
        document.getElementById('hashrate').textContent = formatHashrate(data.hashrate);
        // ... update other fields
    } catch (error) {
        console.error('Error:', error);
    }
}

// Call every 30 seconds
setInterval(fetchBlockchainInfo, 30000);
```

## 🌐 Deploy to Production

### Option 1: Nginx
```nginx
server {
    listen 80;
    server_name dashboard.bitquan.io;
    
    root /opt/bitquan/web-dashboard/public;
    index index.html;
    
    location / {
        try_files $uri $uri/ =404;
    }
}
```

### Option 2: Apache
```apache
<VirtualHost *:80>
    ServerName dashboard.bitquan.io
    DocumentRoot /opt/bitquan/web-dashboard/public
    
    <Directory /opt/bitquan/web-dashboard/public>
        AllowOverride All
        Require all granted
    </Directory>
</VirtualHost>
```

### Option 3: Vercel/Netlify
```bash
# Just drag & drop the 'public' folder!
```

## 📊 API Integration

The dashboard expects these API endpoints:

```
GET /api/blockchain/info
{
    "height": 123,
    "hashrate": 500000,
    "difficulty": 1000,
    "nodes": 5,
    "transactions": 456
}

GET /api/blocks/recent?limit=10
[
    {
        "height": 123,
        "hash": "00000...",
        "time": 1699999999,
        "txCount": 2
    },
    ...
]
```

## 🎨 Customization

### Change Colors
Edit the CSS gradient in `index.html`:
```css
background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
```

### Add More Cards
Copy a card div and customize:
```html
<div class="card">
    <div class="card-title">
        <span class="card-icon">🎯</span>
        Your Metric
    </div>
    <div class="card-value" id="yourMetric">-</div>
    <div class="card-label">Description</div>
</div>
```

## 🔒 Security

For production:
- ✅ Use HTTPS
- ✅ Add rate limiting
- ✅ Sanitize all inputs
- ✅ Enable CORS properly
- ✅ Add authentication (if needed)

## 📱 Mobile Support

The dashboard is fully responsive and works on:
- 📱 Mobile phones
- 📲 Tablets
- 💻 Desktops
- 🖥️ Large screens

## 🐛 Troubleshooting

### Dashboard shows "-" for all values
- Check if RPC endpoint is correct
- Verify node is running
- Check browser console for errors

### CORS errors
- Make sure server sends CORS headers
- Use the provided Python server
- Or configure your web server properly

### Blocks not updating
- Check auto-refresh is enabled
- Verify API endpoints are working
- Check browser console logs

## 📞 Support

Issues with the dashboard?
- 🐛 Report: https://github.com/AlphaB135/BitQuan/issues
- 💬 Discord: #dashboard-help
- 📧 Email: support@bitquan.io

## 📄 License

Apache 2.0 - See main repo LICENSE

---

**Built with ❤️ for BitQuan Testnet**
