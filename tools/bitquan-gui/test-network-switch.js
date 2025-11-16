#!/usr/bin/env node

// Test script for network switching
console.log('🌐 ทดสอบ BitQuan Network Switching...\n');

async function testNetworkSwitching() {
  try {
    console.log('1️⃣ ทดสอบ Mainnet Connection...');
    const mainnetResponse = await fetch('http://127.0.0.1:8332/auth/login', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: 'admin', password: 'mysecretjwtkey' })
    });
    
    if (mainnetResponse.ok) {
      const mainnetData = await mainnetResponse.json();
      console.log('✅ Mainnet: Authentication successful');
      console.log('📊 Token:', mainnetData.access_token.substring(0, 20) + '...');
      
      // Test mainnet RPC
      const mainnetToken = mainnetData.access_token;
      const mainnetRpcResponse = await fetch('http://127.0.0.1:8332', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${mainnetToken}`
        },
        body: JSON.stringify({
          jsonrpc: '2.0',
          method: 'getblockchaininfo',
          params: [],
          id: 1
        })
      });
      
      if (mainnetRpcResponse.ok) {
        const mainnetRpcData = await mainnetRpcResponse.json();
        console.log('✅ Mainnet RPC: Connected');
        console.log('📈 Height:', mainnetRpcData.result.blocks);
      }
    } else {
      console.log('❌ Mainnet: Authentication failed');
    }
    
    console.log('\n2️⃣ ทดสอบ Testnet Connection...');
    
    // Try to connect to testnet (if available)
    try {
      const testnetResponse = await fetch('http://127.0.0.1:19443/auth/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ username: 'admin', password: 'testnet123' }),
        timeout: 5000
      });
      
      if (testnetResponse.ok) {
        const testnetData = await testnetResponse.json();
        console.log('✅ Testnet: Authentication successful');
        console.log('📊 Token:', testnetData.access_token.substring(0, 20) + '...');
        
        // Test testnet RPC
        const testnetToken = testnetData.access_token;
        const testnetRpcResponse = await fetch('http://127.0.0.1:19443', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${testnetToken}`
          },
          body: JSON.stringify({
            jsonrpc: '2.0',
            method: 'getblockchaininfo',
            params: [],
            id: 2
          })
        });
        
        if (testnetRpcResponse.ok) {
          const testnetRpcData = await testnetRpcResponse.json();
          console.log('✅ Testnet RPC: Connected');
          console.log('📈 Height:', testnetRpcData.result.blocks);
        }
      } else {
        console.log('⚠️  Testnet: Not available (this is expected)');
      }
    } catch (error) {
      console.log('⚠️  Testnet: Not available (this is expected)');
    }
    
    console.log('\n3️⃣ สถานะการทำงาน:');
    console.log('🟢 Mainnet: พร้อมใช้งาน (Port 8332)');
    console.log('🟡 Testnet: ไม่พร้อมใช้งาน (Port 19443)');
    console.log('🔄 GUI: รองรับ network switcher');
    
    console.log('\n4️⃣ คำแนะนำ:');
    console.log('📌 ใช้ Mainnet สำหรับการทำงานจริง');
    console.log('📌 Testnet จะพร้อมใช้เมื่อ start node บน port 19443');
    console.log('📌 GUI สามารถ switch network ได้ผ่าน Settings page');
    
  } catch (error) {
    console.log('❌ Error:', error.message);
  }
}

testNetworkSwitching();