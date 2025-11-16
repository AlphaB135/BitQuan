#!/usr/bin/env node

const { invoke } = require('@tauri-apps/api');

async function testRPC() {
  try {
    console.log('Testing get_balances...');
    const balances = await invoke('get_balances');
    console.log('Balances:', balances);
    
    console.log('Testing get_miners...');
    const miners = await invoke('get_miners');
    console.log('Miners:', miners);
    
  } catch (error) {
    console.error('Error:', error);
  }
}

testRPC();