http://localhost:3002/
les deux fichiers existent mais j ai cette erreur
[plugin:vite:import-analysis] Failed to resolve import "./mpptAnimations.css" from "src/components/nodes/MPPTNode.jsx". Does the file exist?
C:/reactflow-energie/src/components/nodes/MPPTNode.jsx:2:7
16 |  }
17 |  import { Handle, Position } from "@xyflow/react";
18 |  import "./mpptAnimations.css";
   |          ^
19 |  const MPPTNode = ({ id, data }) => {
20 |    const {
    at TransformPluginContext._formatError (file:///C:/reactflow-energie/node_modules/vite/dist/node/chunks/dep-BK3b2jBa.js:49258:41)
    at TransformPluginContext.error (file:///C:/reactflow-energie/node_modules/vite/dist/node/chunks/dep-BK3b2jBa.js:49253:16)
    at normalizeUrl (file:///C:/reactflow-energie/node_modules/vite/dist/node/chunks/dep-BK3b2jBa.js:64307:23)
    at process.processTicksAndRejections (node:internal/process/task_queues:103:5)
    at async file:///C:/reactflow-energie/node_modules/vite/dist/node/chunks/dep-BK3b2jBa.js:64439:39
    at async Promise.all (index 4)
    at async TransformPluginContext.transform (file:///C:/reactflow-energie/node_modules/vite/dist/node/chunks/dep-BK3b2jBa.js:64366:7)
    at async PluginContainer.transform (file:///C:/reactflow-energie/node_modules/vite/dist/node/chunks/dep-BK3b2jBa.js:49099:18)
    at async loadAndTransform (file:///C:/reactflow-energie/node_modules/vite/dist/node/chunks/dep-BK3b2jBa.js:51978:27)
    at async viteTransformMiddleware (file:///C:/reactflow-energie/node_modules/vite/dist/node/chunks/dep-BK3b2jBa.js:62106:24
Click outside, press Esc key, or fix the code to dismiss.
You can also disable this overlay by setting server.hmr.overlay to false in vite.config.js.
