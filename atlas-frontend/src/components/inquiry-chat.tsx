'use client';

import { useState, useEffect, useRef } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Badge } from '@/components/ui/badge';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Send, MessageCircle, Loader2, Sparkles, Zap, Lightbulb, Edit3, Check, X } from 'lucide-react';
import { MarketplaceService, InquiryAssistantService } from '@/lib/services';
import { InquiryMessage } from '@/types/pharmaceutical';
import { InquirySuggestion, SuggestionType, AssistantQuotaStatus } from '@/types/inquiry-assistant';
import { useAuthStore } from '@/lib/auth-store';
import { toast } from 'react-toastify';
import { format } from 'date-fns';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

interface InquiryChatProps {
  inquiryId: string;
  buyerCompany: string;
  sellerCompany: string;
  isSeller?: boolean;
  onMessageSent?: () => void;
}

const SUGGESTION_TYPES: { value: SuggestionType; label: string; description: string }[] = [
  { value: 'initial_response', label: 'Initial Response', description: 'Welcoming first reply to inquiry' },
  { value: 'negotiation', label: 'Negotiation', description: 'Find middle ground, suggest alternatives' },
  { value: 'pricing_adjustment', label: 'Pricing', description: 'Justify price or offer discount' },
  { value: 'terms_clarification', label: 'Terms', description: 'Clarify payment, delivery, compliance' },
  { value: 'closing_deal', label: 'Close Deal', description: 'Finalize and confirm agreement' },
  { value: 'follow_up', label: 'Follow Up', description: 'Check in on their decision' },
  { value: 'rejection', label: 'Decline', description: 'Politely reject inquiry' },
];

export function InquiryChat({ inquiryId, buyerCompany, sellerCompany, isSeller = false, onMessageSent }: InquiryChatProps) {
  const { user } = useAuthStore();
  const [messages, setMessages] = useState<InquiryMessage[]>([]);
  const [newMessage, setNewMessage] = useState('');
  const [isLoading, setIsLoading] = useState(true);
  const [isSending, setIsSending] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  // AI Suggestion State
  const [currentSuggestion, setCurrentSuggestion] = useState<InquirySuggestion | null>(null);
  const [selectedType, setSelectedType] = useState<SuggestionType>('initial_response');
  const [isGenerating, setIsGenerating] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [editedText, setEditedText] = useState('');
  const [quota, setQuota] = useState<AssistantQuotaStatus | null>(null);
  const [showSuggestionUI, setShowSuggestionUI] = useState(false);

  // Load messages and quota
  useEffect(() => {
    loadMessages();
    if (isSeller) {
      loadQuota();
    }
  }, [inquiryId, isSeller]);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages]);

  const loadMessages = async () => {
    try {
      setIsLoading(true);
      const data = await MarketplaceService.getInquiryMessages(inquiryId);
      setMessages(data);
    } catch (error) {
      console.error('Failed to load messages:', error);
      toast.error('Failed to load conversation');
    } finally {
      setIsLoading(false);
    }
  };

  const loadQuota = async () => {
    try {
      const data = await InquiryAssistantService.getQuota();
      setQuota(data);
    } catch (error) {
      console.error('Failed to load quota:', error);
    }
  };

  const handleGenerateSuggestion = async () => {
    if (!isSeller) {
      toast.error('AI suggestions are only available for sellers');
      return;
    }

    if (quota && quota.assists_remaining <= 0) {
      toast.error('Monthly AI assist limit reached. Please upgrade or wait for reset.');
      return;
    }

    try {
      setIsGenerating(true);
      const suggestion = await InquiryAssistantService.generateSuggestion(inquiryId, {
        suggestion_type: selectedType,
      });

      setCurrentSuggestion(suggestion);
      setEditedText(suggestion.suggestion_text);
      setIsEditing(false);
      setShowSuggestionUI(true);
      await loadQuota(); // Refresh quota

      toast.success('AI suggestion generated!');
    } catch (error) {
      console.error('Failed to generate suggestion:', error);
      toast.error(error instanceof Error ? error.message : 'Failed to generate suggestion');
    } finally {
      setIsGenerating(false);
    }
  };

  const handleAcceptSuggestion = async (useEdited: boolean = false) => {
    if (!currentSuggestion) return;

    try {
      setIsSending(true);
      await InquiryAssistantService.acceptSuggestion(
        currentSuggestion.id,
        useEdited ? { edited_text: editedText } : {}
      );

      // Reload messages to show the new message
      await loadMessages();

      // Clear suggestion state
      setCurrentSuggestion(null);
      setShowSuggestionUI(false);
      setEditedText('');
      setIsEditing(false);

      toast.success(useEdited ? 'Edited message sent!' : 'Suggestion sent!');
      onMessageSent?.();
    } catch (error) {
      console.error('Failed to send suggestion:', error);
      toast.error(error instanceof Error ? error.message : 'Failed to send message');
    } finally {
      setIsSending(false);
    }
  };

  const handleCancelSuggestion = () => {
    setCurrentSuggestion(null);
    setShowSuggestionUI(false);
    setEditedText('');
    setIsEditing(false);
  };

  const handleSendMessage = async () => {
    if (!newMessage.trim()) return;

    try {
      setIsSending(true);
      const message = await MarketplaceService.sendInquiryMessage(inquiryId, newMessage.trim());
      setMessages([...messages, message]);
      setNewMessage('');
      onMessageSent?.();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to send message');
    } finally {
      setIsSending(false);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  };

  const isMyMessage = (message: InquiryMessage) => {
    return message.sender_id === user?.id;
  };

  if (isLoading) {
    return (
      <Card>
        <CardContent className="flex items-center justify-center py-8">
          <Loader2 className="h-6 w-6 animate-spin text-blue-600" />
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <MessageCircle className="h-5 w-5" />
              Negotiation Chat
            </CardTitle>
            <div className="text-sm text-gray-600">
              Between {buyerCompany} (Buyer) and {sellerCompany} (Seller)
            </div>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Messages Area */}
        <div className="h-96 overflow-y-auto pr-4" ref={scrollRef}>
          <div className="space-y-4">
            {messages.length === 0 ? (
              <div className="text-center py-8 text-gray-500">
                <MessageCircle className="h-12 w-12 mx-auto mb-3 opacity-30" />
                <p>No messages yet. Start the negotiation!</p>
              </div>
            ) : (
              messages.map((message) => (
                <div
                  key={message.id}
                  className={`flex ${isMyMessage(message) ? 'justify-end' : 'justify-start'}`}
                >
                  <div
                    className={`max-w-[70%] rounded-lg px-4 py-2 ${
                      isMyMessage(message)
                        ? 'bg-blue-600 text-white'
                        : 'bg-gray-100 text-gray-900'
                    }`}
                  >
                    <div className="flex items-center gap-2 mb-1">
                      <span className="text-xs font-semibold">
                        {message.sender_company}
                      </span>
                      <span className={`text-xs ${isMyMessage(message) ? 'text-blue-100' : 'text-gray-500'}`}>
                        {format(new Date(message.created_at), 'MMM d, h:mm a')}
                      </span>
                    </div>
                    <div className={`text-sm prose prose-sm max-w-none ${
                      isMyMessage(message)
                        ? 'prose-invert prose-p:my-1 prose-strong:text-white prose-ul:my-1 prose-li:my-0.5'
                        : 'prose-p:my-1 prose-strong:text-gray-900 prose-ul:my-1 prose-li:my-0.5'
                    }`}>
                      <ReactMarkdown remarkPlugins={[remarkGfm]}>
                        {message.message}
                      </ReactMarkdown>
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        {/* AI Suggestion UI - Only for sellers */}
        {isSeller && (
          <>
            {/* AI Suggestion Generator */}
            {!showSuggestionUI ? (
              <div className="space-y-3 border-t pt-4">
                <div className="flex items-center gap-2 text-sm font-medium text-gray-700">
                  <Sparkles className="h-4 w-4 text-purple-600" />
                  AI-Powered Response Suggestions
                </div>
                <div className="flex gap-2">
                  <Select value={selectedType} onValueChange={(value) => setSelectedType(value as SuggestionType)}>
                    <SelectTrigger className="flex-1">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {SUGGESTION_TYPES.map((type) => (
                        <SelectItem key={type.value} value={type.value}>
                          <div className="flex flex-col">
                            <span className="font-medium">{type.label}</span>
                            <span className="text-xs text-gray-500">{type.description}</span>
                          </div>
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <Button
                    onClick={handleGenerateSuggestion}
                    disabled={isGenerating || (quota && quota.assists_remaining <= 0)}
                    className="bg-purple-600 hover:bg-purple-700"
                  >
                    {isGenerating ? (
                      <>
                        <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                        Generating...
                      </>
                    ) : (
                      <>
                        <Sparkles className="h-4 w-4 mr-2" />
                        Generate
                      </>
                    )}
                  </Button>
                </div>
              </div>
            ) : (
              /* AI Suggestion Display */
              currentSuggestion && (
                <div className="space-y-3 border-t pt-4">
                  <Alert className="bg-gradient-to-r from-purple-50 to-blue-50 border-purple-200">
                    <Sparkles className="h-4 w-4 text-purple-600" />
                    <AlertDescription>
                      <div className="space-y-3">
                        <div className="flex items-center justify-between">
                          <span className="font-semibold text-purple-900">AI Suggestion Ready</span>
                          <div className="flex items-center gap-2 text-xs text-gray-600">
                            <span className="flex items-center gap-1">
                              <Lightbulb className="h-3 w-3" />
                              {currentSuggestion.suggestion_type.replace('_', ' ').toUpperCase()}
                            </span>
                            <span>${currentSuggestion.ai_cost_usd}</span>
                          </div>
                        </div>

                        {/* Suggestion Text */}
                        {!isEditing ? (
                          <div className="bg-white rounded-lg p-3 border border-purple-100">
                            <div className="prose prose-sm max-w-none prose-p:my-2 prose-strong:font-bold prose-strong:text-gray-900 prose-ul:my-2 prose-li:my-1">
                              <ReactMarkdown remarkPlugins={[remarkGfm]}>
                                {editedText}
                              </ReactMarkdown>
                            </div>
                          </div>
                        ) : (
                          <Textarea
                            value={editedText}
                            onChange={(e) => setEditedText(e.target.value)}
                            className="min-h-[120px] font-sans"
                            placeholder="Edit the suggestion..."
                          />
                        )}

                        {/* AI Reasoning */}
                        {currentSuggestion.reasoning && !isEditing && (
                          <div className="text-xs text-gray-600 bg-white/50 rounded p-2 border border-purple-100">
                            <span className="font-semibold">💡 AI Reasoning: </span>
                            {currentSuggestion.reasoning}
                          </div>
                        )}

                        {/* Actions */}
                        <div className="flex gap-2">
                          {!isEditing ? (
                            <>
                              <Button
                                onClick={() => handleAcceptSuggestion(false)}
                                disabled={isSending}
                                className="bg-green-600 hover:bg-green-700 flex-1"
                              >
                                {isSending ? (
                                  <Loader2 className="h-4 w-4 animate-spin mr-2" />
                                ) : (
                                  <Check className="h-4 w-4 mr-2" />
                                )}
                                Send as is
                              </Button>
                              <Button
                                onClick={() => setIsEditing(true)}
                                variant="outline"
                                className="flex-1"
                              >
                                <Edit3 className="h-4 w-4 mr-2" />
                                Edit & Send
                              </Button>
                              <Button
                                onClick={handleCancelSuggestion}
                                variant="outline"
                                size="icon"
                              >
                                <X className="h-4 w-4" />
                              </Button>
                            </>
                          ) : (
                            <>
                              <Button
                                onClick={() => handleAcceptSuggestion(true)}
                                disabled={isSending || !editedText.trim()}
                                className="bg-green-600 hover:bg-green-700 flex-1"
                              >
                                {isSending ? (
                                  <Loader2 className="h-4 w-4 animate-spin mr-2" />
                                ) : (
                                  <Send className="h-4 w-4 mr-2" />
                                )}
                                Send Edited
                              </Button>
                              <Button
                                onClick={() => {
                                  setIsEditing(false);
                                  setEditedText(currentSuggestion.suggestion_text);
                                }}
                                variant="outline"
                              >
                                Cancel Edit
                              </Button>
                            </>
                          )}
                        </div>
                      </div>
                    </AlertDescription>
                  </Alert>
                </div>
              )
            )}
          </>
        )}

        {/* Manual Message Input */}
        <div className="flex gap-2">
          <Input
            value={newMessage}
            onChange={(e) => setNewMessage(e.target.value)}
            onKeyPress={handleKeyPress}
            placeholder="Type your message..."
            disabled={isSending}
            className="flex-1"
          />
          <Button
            onClick={handleSendMessage}
            disabled={isSending || !newMessage.trim()}
            size="icon"
          >
            {isSending ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Send className="h-4 w-4" />
            )}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
