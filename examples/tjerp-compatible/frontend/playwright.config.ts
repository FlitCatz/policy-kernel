import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './e2e', workers: 1, retries: 0,
  reporter: [['json', {outputFile: '.policy-reports/e2e.json'}]],
  use: {baseURL:'http://127.0.0.1:4180', headless:true},
  webServer: [
    {command:'dotnet run --no-restore --project src/Api --urls http://127.0.0.1:5077', cwd:'../backend', url:'http://127.0.0.1:5077/api/greeting', reuseExistingServer:false},
    {command:'node node_modules/vite/bin/vite.js', url:'http://127.0.0.1:4180', reuseExistingServer:false},
  ],
});
