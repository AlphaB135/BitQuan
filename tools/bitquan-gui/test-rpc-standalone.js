#!/usr/bin/env node

// Test script to verify RPC functions work

async function testRPCFunctions() {
  console.log('Testing RPC functions...');
  
  // Test 1: Check if BitQuan node is running
  console.log('\n1. Testing BitQuan node connectivity...');
  try {
    const response = await fetch('http://127.0.0.1:8332/auth/login', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: 'admin', password: 'mysecretjwtkey' })
    });
    
    if (response.ok) {
      const data = await response.json();
      console.log('✅ JWT Login successful');
      console.log('Token:', data.access_token.substring(0, 20) + '...');
      
      // Test 2: Test actual RPC call
      console.log('\n2. Testing RPC call...');
      const rpcResponse = await fetch('http://127.0.0.1:8332', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${data.access_token}`
        },
        body: JSON.stringify({
          jsonrpc: '2.0',
          method: 'getpoolstats',
          params: [],
          id: 1
        })
      });
      
      if (rpcResponse.ok) {
        const rpcData = await rpcResponse.json();
        console.log('✅ RPC call successful');
        console.log('Pool stats:', JSON.stringify(rpcData.result, null, 2));
      } else {
        console.log('❌ RPC call failed:', rpcResponse.status);
      }
    } else {
      console.log('❌ JWT Login failed:', response.status);
    }
  } catch (error) {
    console.log('❌ Connection failed:', error.message);
  }
}

testRPCFunctions();