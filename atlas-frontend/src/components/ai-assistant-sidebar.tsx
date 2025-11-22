'use client';

import { useState, useEffect, useRef } from 'react';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import {
  Sparkles,
  Send,
  X,
  Loader2,
  Star,
  History,
  Code,
  ChevronDown,
  ChevronUp,
  Clock,
  DollarSign,
  Zap,
  Check,
} from 'lucide-react';
import { NlQueryService } from '@/lib/services/nl-query-service';
import type { QueryResponse, QuotaStatus, FavoriteQuery } from '@/types/nl-query';
import { toast } from 'react-toastify';
import { cn } from '@/lib/utils';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

interface Message {
  id: string;
  type: 'user' | 'assistant' | 'system';
  content: string;
  queryResponse?: QueryResponse;
  timestamp: Date;
}

interface AiAssistantSidebarProps {
  isOpen: boolean;
  onClose: () => void;
}

export function AiAssistantSidebar({ isOpen, onClose }: AiAssistantSidebarProps) {
  const [messages, setMessages] = useState<Message[]>([
    {
      id: '1',
      type: 'system',
      content: "Hi! I'm your AI data assistant. Ask me anything about your inventory, transactions, or analytics. Try: \"Show me products expiring soon\" or \"What are my top-selling items?\"",
      timestamp: new Date(),
    },
  ]);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [quota, setQuota] = useState<QuotaStatus | null>(null);
  const [favorites, setFavorites] = useState<FavoriteQuery[]>([]);
  const [showSqlFor, setShowSqlFor] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Load quota and favorites on mount
  useEffect(() => {
    if (isOpen) {
      loadQuota();
      loadFavorites();
      inputRef.current?.focus();
    }
  }, [isOpen]);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages]);

  const loadQuota = async () => {
    try {
      const data = await NlQueryService.getQuota();
      setQuota(data);
    } catch (error) {
      console.error('Failed to load quota:', error);
    }
  };

  const loadFavorites = async () => {
    try {
      const data = await NlQueryService.getFavorites();
      setFavorites(data);
    } catch (error) {
      console.error('Failed to load favorites:', error);
    }
  };

  const formatResults = (results: any[] | null | undefined, count: number): string => {
    if (!results || results.length === 0) {
      return `No results found.`;
    }

    let summary = `Found ${count} item${count !== 1 ? 's' : ''}:\n\n`;

    results.slice(0, 10).forEach((item, index) => {
      // Format inventory items nicely
      if (item.brand_name || item.generic_name) {
        summary += `${index + 1}. ${item.brand_name || item.generic_name}`;
        if (item.manufacturer) summary += ` by ${item.manufacturer}`;
        if (item.strength) summary += ` - ${item.strength}`;
        if (item.quantity) summary += `\n   • Quantity: ${item.quantity} units`;
        if (item.batch_number) summary += `\n   • Batch: ${item.batch_number}`;
        if (item.expiry_date) summary += `\n   • Expires: ${new Date(item.expiry_date).toLocaleDateString()}`;
        if (item.unit_price) summary += `\n   • Price: $${parseFloat(item.unit_price).toFixed(2)}`;
        if (item.storage_location) summary += `\n   • Location: ${item.storage_location}`;
        summary += '\n\n';
      } else {
        // Generic formatting for other query results
        const keys = Object.keys(item).slice(0, 5);
        summary += `${index + 1}. `;
        keys.forEach((key, i) => {
          const value = item[key];
          if (value !== null && value !== undefined) {
            summary += `${key}: ${value}`;
            if (i < keys.length - 1) summary += ', ';
          }
        });
        summary += '\n';
      }
    });

    if (count > 10) {
      summary += `\n...and ${count - 10} more items`;
    }

    return summary;
  };

  const handleSend = async () => {
    if (!input.trim() || isLoading) return;

    const userMessage: Message = {
      id: Date.now().toString(),
      type: 'user',
      content: input.trim(),
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, userMessage]);
    setInput('');
    setIsLoading(true);

    try {
      const response = await NlQueryService.executeQuery(input.trim());
      console.log('NL Query Response:', response);

      // Create assistant message based on response type
      let content = '';
      if (response.status === 'success') {
        if (response.response_type === 'conversation') {
          // Conversational response - just show the AI's answer
          content = response.ai_response || 'No response received';
        } else {
          // SQL query response - format the data results
          content = formatResults(response.results, response.result_count || 0);
        }
      } else {
        content = response.error_message || 'Query failed';
      }

      const assistantMessage: Message = {
        id: response.id,
        type: 'assistant',
        content,
        queryResponse: response,
        timestamp: new Date(),
      };

      setMessages((prev) => [...prev, assistantMessage]);
      loadQuota(); // Refresh quota after query
    } catch (error: any) {
      console.error('NL Query Error:', error);
      console.error('Error message:', error.message);
      console.error('Error response:', error.response);

      const errorMessage: Message = {
        id: Date.now().toString() + '-error',
        type: 'system',
        content: error.message || error.response?.data?.error || 'Failed to execute query. Please try again.',
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, errorMessage]);
      toast.error('Query failed');
    } finally {
      setIsLoading(false);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleQuickQuery = (query: string) => {
    setInput(query);
    inputRef.current?.focus();
  };

  const handleSaveFavorite = async (query: string) => {
    try {
      await NlQueryService.saveFavorite({ query_text: query });
      loadFavorites();
      toast.success('Saved to favorites');
    } catch (error) {
      toast.error('Failed to save favorite');
    }
  };

  const quickQueries = [
    "Show me all inventory items",
    "Products expiring in 30 days",
    "Top 5 best-selling products",
    "Low stock items (under 50 units)",
    "Pending inquiries",
  ];

  if (!isOpen) return null;

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 bg-black/20 backdrop-blur-sm z-40 transition-opacity"
        onClick={onClose}
      />

      {/* Sidebar */}
      <div className="fixed right-0 top-0 h-full w-[600px] bg-background border-l shadow-2xl z-50 flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b bg-gradient-to-r from-purple-500/10 to-blue-500/10">
          <div className="flex items-center gap-2">
            <div className="p-2 rounded-lg bg-purple-500/20">
              <Sparkles className="h-5 w-5 text-purple-500" />
            </div>
            <div>
              <h2 className="font-semibold text-lg">AI Data Assistant</h2>
              <p className="text-xs text-muted-foreground">Ask anything about your data</p>
            </div>
          </div>
          <Button variant="ghost" size="icon" onClick={onClose}>
            <X className="h-4 w-4" />
          </Button>
        </div>

        {/* Messages */}
        <div className="flex-1 p-4 overflow-y-auto" ref={scrollRef}>
          <div className="space-y-4">
            {messages.map((message) => (
              <div
                key={message.id}
                className={cn(
                  'flex',
                  message.type === 'user' ? 'justify-end' : 'justify-start'
                )}
              >
                <div
                  className={cn(
                    'max-w-[85%] rounded-2xl px-4 py-2.5 space-y-2',
                    message.type === 'user' &&
                      'bg-purple-500 text-white',
                    message.type === 'assistant' &&
                      'bg-muted border',
                    message.type === 'system' &&
                      'bg-blue-50 text-blue-900 border border-blue-200'
                  )}
                >
                  {/* Message content */}
                  {message.type === 'user' ? (
                    <p className="text-sm whitespace-pre-wrap text-white">{message.content}</p>
                  ) : (
                    <div className="text-sm prose prose-sm max-w-none prose-p:my-2 prose-strong:font-semibold prose-strong:text-foreground prose-ul:my-2 prose-li:my-1">
                      <ReactMarkdown remarkPlugins={[remarkGfm]}>
                        {message.content}
                      </ReactMarkdown>
                    </div>
                  )}

                  {/* Query response details */}
                  {message.queryResponse && message.queryResponse.status === 'success' && (
                    <div className="space-y-2 pt-2">
                      {/* Stats */}
                      <div className="flex items-center gap-3 text-xs">
                        <span className="flex items-center gap-1 text-muted-foreground">
                          <Clock className="h-3 w-3" />
                          {message.queryResponse.execution_time_ms}ms
                        </span>
                        <span className="flex items-center gap-1 text-muted-foreground">
                          <DollarSign className="h-3 w-3" />
                          ${message.queryResponse.ai_cost_usd}
                        </span>
                      </div>

                      {/* SQL Toggle - only show for SQL queries */}
                      {message.queryResponse.response_type === 'sql_query' && message.queryResponse.generated_sql && (
                        <>
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-7 text-xs w-full justify-between"
                            onClick={() =>
                              setShowSqlFor(
                                showSqlFor === message.id ? null : message.id
                              )
                            }
                          >
                            <span className="flex items-center gap-1.5">
                              <Code className="h-3 w-3" />
                              View SQL
                            </span>
                            {showSqlFor === message.id ? (
                              <ChevronUp className="h-3 w-3" />
                            ) : (
                              <ChevronDown className="h-3 w-3" />
                            )}
                          </Button>

                          {/* SQL Display */}
                          {showSqlFor === message.id && (
                            <div className="bg-slate-900 text-green-400 p-3 rounded-lg text-xs font-mono overflow-x-auto">
                              {message.queryResponse.generated_sql}
                            </div>
                          )}
                        </>
                      )}

                      {/* Actions */}
                      <div className="flex gap-2">
                        <Button
                          variant="outline"
                          size="sm"
                          className="h-7 text-xs flex-1"
                          onClick={() => handleSaveFavorite(message.queryResponse!.query_text)}
                        >
                          <Star className="h-3 w-3 mr-1" />
                          Save
                        </Button>
                      </div>
                    </div>
                  )}

                  {/* Timestamp */}
                  <div className="text-[10px] opacity-60 mt-1">
                    {message.timestamp.toLocaleTimeString()}
                  </div>
                </div>
              </div>
            ))}

            {/* Loading indicator */}
            {isLoading && (
              <div className="flex justify-start">
                <div className="bg-muted border rounded-2xl px-4 py-3">
                  <div className="flex items-center gap-2 text-sm text-muted-foreground">
                    <Loader2 className="h-4 w-4 animate-spin" />
                    Analyzing your question...
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Quick Queries (show when no messages yet) */}
        {messages.length === 1 && (
          <div className="px-4 pb-3 space-y-2">
            <p className="text-xs text-muted-foreground font-medium mb-2">Try these:</p>
            <div className="flex flex-wrap gap-2">
              {quickQueries.slice(0, 3).map((query, i) => (
                <Button
                  key={i}
                  variant="outline"
                  size="sm"
                  className="text-xs h-7"
                  onClick={() => handleQuickQuery(query)}
                >
                  {query}
                </Button>
              ))}
            </div>
          </div>
        )}

        {/* Favorites (show when we have them) */}
        {favorites && favorites.length > 0 && messages.length > 1 && (
          <div className="px-4 pb-3 border-t pt-3">
            <p className="text-xs text-muted-foreground font-medium mb-2 flex items-center gap-1.5">
              <Star className="h-3 w-3" />
              Favorites
            </p>
            <div className="flex flex-wrap gap-2">
              {favorites.slice(0, 3).map((fav) => (
                <Button
                  key={fav.id}
                  variant="outline"
                  size="sm"
                  className="text-xs h-7"
                  onClick={() => handleQuickQuery(fav.query_text)}
                >
                  {fav.query_text.slice(0, 30)}...
                </Button>
              ))}
            </div>
          </div>
        )}

        {/* Input Area */}
        <div className="p-4 border-t bg-muted/30">
          <div className="flex gap-2">
            <Input
              ref={inputRef}
              placeholder="Ask about your inventory, sales, or analytics..."
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyPress={handleKeyPress}
              disabled={isLoading}
              className="flex-1"
            />
            <Button
              onClick={handleSend}
              disabled={!input.trim() || isLoading}
              size="icon"
              className="bg-purple-500 hover:bg-purple-600"
            >
              {isLoading ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Send className="h-4 w-4" />
              )}
            </Button>
          </div>
          <p className="text-[10px] text-muted-foreground mt-2 text-center">
            Powered by Claude AI ✨
          </p>
        </div>
      </div>
    </>
  );
}
