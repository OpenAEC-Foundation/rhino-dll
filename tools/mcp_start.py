# -*- coding: utf-8 -*-
# Starts the rhinomcp bridge in this (hidden) Rhino on the port in RHINO_MCP_PORT (1999).
import os
import Rhino
port = os.environ.get("RHINO_MCP_PORT", "1999")
Rhino.RhinoApp.RunScript("_-mcpstart _Port=%s _Enter" % port, False)
