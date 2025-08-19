#!/usr/bin/env python3
"""
Simple test MCP server for testing ramparts with Azure OpenAI configuration
"""

import json
import sys
from http.server import HTTPServer, BaseHTTPRequestHandler
import threading
import time

class MCPHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        if self.path == '/':
            content_length = int(self.headers['Content-Length'])
            post_data = self.rfile.read(content_length)
            
            try:
                request = json.loads(post_data.decode('utf-8'))
                
                if request.get('method') == 'initialize':
                    response = {
                        "jsonrpc": "2.0",
                        "id": request.get('id'),
                        "result": {
                            "protocolVersion": "2024-11-05",
                            "capabilities": {
                                "tools": {},
                                "resources": {}
                            },
                            "serverInfo": {
                                "name": "test-mcp-server",
                                "version": "1.0.0"
                            }
                        }
                    }
                elif request.get('method') == 'tools/list':
                    response = {
                        "jsonrpc": "2.0",
                        "id": request.get('id'),
                        "result": {
                            "tools": [
                                {
                                    "name": "add_numbers",
                                    "description": "Add two numbers together",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "a": {"type": "number", "description": "First number"},
                                            "b": {"type": "number", "description": "Second number"}
                                        },
                                        "required": ["a", "b"]
                                    }
                                }
                            ]
                        }
                    }
                elif request.get('method') == 'resources/list':
                    response = {
                        "jsonrpc": "2.0",
                        "id": request.get('id'),
                        "result": {
                            "resources": []
                        }
                    }
                else:
                    response = {
                        "jsonrpc": "2.0",
                        "id": request.get('id'),
                        "error": {
                            "code": -32601,
                            "message": "Method not found"
                        }
                    }
                
                self.send_response(200)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps(response).encode('utf-8'))
                
            except Exception as e:
                self.send_response(500)
                self.end_headers()
                self.wfile.write(f"Error: {str(e)}".encode('utf-8'))
    
    def log_message(self, format, *args):
        # Suppress default logging
        pass

def start_server():
    server = HTTPServer(('localhost', 8000), MCPHandler)
    print("Test MCP server started on http://localhost:8000")
    server.serve_forever()

if __name__ == "__main__":
    start_server()
