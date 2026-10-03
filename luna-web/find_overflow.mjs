import { spawn } from 'child_process';
import http from 'http';

const edgePath = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const edge = spawn(edgePath, [
  '--headless',
  '--remote-debugging-port=9222',
  '--window-size=375,812',
  'http://localhost:4321/'
]);

// Wait 2s for Edge to start
setTimeout(async () => {
  try {
    const list = await fetch('http://127.0.0.1:9222/json/list').then(r => r.json());
    console.log('Target pages:', list.length);
    const target = list[0];
    const wsUrl = target.webSocketDebuggerUrl;
    console.log('WS URL:', wsUrl);

    // Use WebSocket
    const ws = new WebSocket(wsUrl);
    ws.onopen = async () => {
      ws.send(JSON.stringify({
        id: 0,
        method: 'Emulation.setDeviceMetricsOverride',
        params: {
          width: 375,
          height: 812,
          deviceScaleFactor: 2,
          mobile: true
        }
      }));
      setTimeout(() => {
        ws.send(JSON.stringify({
          id: 1,
          method: 'Runtime.evaluate',
          params: {
            expression: `
              (() => {
                const overflow = [];
                const docWidth = document.documentElement.clientWidth;
                const all = document.querySelectorAll('*');
                for (const el of all) {
                  const rect = el.getBoundingClientRect();
                  if (rect.right > docWidth + 1) {
                    overflow.push({
                      tag: el.tagName,
                      class: el.className,
                      width: Math.round(rect.width),
                      right: Math.round(rect.right),
                      docWidth
                    });
                  }
                }
                return JSON.stringify(overflow.slice(0, 15));
              })()
            `
          }
        }));
      }, 500);
    };

    ws.onmessage = (msg) => {
      const data = JSON.parse(msg.data);
      if (data.id === 1) {
        console.log('Overflow elements:', data.result.result.value);
        ws.close();
        edge.kill();
        process.exit(0);
      }
    };
  } catch (err) {
    console.error('Error:', err);
    edge.kill();
    process.exit(1);
  }
}, 2500);
