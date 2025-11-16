'use client';

import { useState, useEffect } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { AlertService } from '@/lib/services/alert-service';
import { toast } from 'react-toastify';
import { Search, Plus, Trash2, Edit, Bell, BellOff, Loader2, Eye, Building, MapPin } from 'lucide-react';
import Link from 'next/link';

interface Watchlist {
  id: string;
  name: string;
  description?: string;
  search_criteria: Record<string, any>;
  alert_enabled: boolean;
  last_checked_at: string;
  last_match_count: number;
  total_matches_found: number;
  created_at: string;
}

export default function WatchlistPage() {
  const [watchlists, setWatchlists] = useState<Watchlist[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    loadWatchlists();
  }, []);

  const loadWatchlists = async () => {
    try {
      setIsLoading(true);
      const data = await AlertService.getWatchlists();

      // Fetch actual match counts for each watchlist
      const watchlistsWithCounts = await Promise.all(
        data.map(async (watchlist: Watchlist) => {
          try {
            const response = await fetch(`/api/alerts/watchlist/${watchlist.id}/matches`, {
              headers: {
                'Authorization': `Bearer ${localStorage.getItem('token')}`,
              },
            });
            if (response.ok) {
              const matchData = await response.json();
              return { ...watchlist, last_match_count: matchData.count };
            }
          } catch (err) {
            console.error('Failed to fetch matches for watchlist:', err);
          }
          return watchlist;
        })
      );

      setWatchlists(watchlistsWithCounts);
    } catch (error) {
      console.error('Failed to load watchlists:', error);
      toast.error('Failed to load watchlists');
    } finally {
      setIsLoading(false);
    }
  };

  const handleDelete = async (id: string, name: string) => {
    if (!confirm(`Are you sure you want to delete the watchlist "${name}"?`)) {
      return;
    }

    try {
      await AlertService.deleteWatchlist(id);
      setWatchlists(watchlists.filter(w => w.id !== id));
      toast.success('Watchlist deleted successfully');
    } catch (error) {
      console.error('Failed to delete watchlist:', error);
      toast.error('Failed to delete watchlist');
    }
  };

  const handleToggleAlerts = async (id: string, currentState: boolean) => {
    try {
      await AlertService.updateWatchlist(id, { alert_enabled: !currentState });
      setWatchlists(watchlists.map(w =>
        w.id === id ? { ...w, alert_enabled: !currentState } : w
      ));
      toast.success(
        !currentState ? 'Alerts enabled for this watchlist' : 'Alerts disabled for this watchlist'
      );
    } catch (error) {
      console.error('Failed to toggle alerts:', error);
      toast.error('Failed to update watchlist');
    }
  };

  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const hours = Math.floor(diff / (1000 * 60 * 60));

    if (hours < 1) return 'Just now';
    if (hours < 24) return `${hours} hour${hours === 1 ? '' : 's'} ago`;
    const days = Math.floor(hours / 24);
    if (days < 7) return `${days} day${days === 1 ? '' : 's'} ago`;
    return date.toLocaleDateString();
  };

  const buildMarketplaceUrl = (criteria: Record<string, any>) => {
    const params = new URLSearchParams();

    if (criteria.search_term) params.set('search', criteria.search_term);
    if (criteria.manufacturers && Array.isArray(criteria.manufacturers)) {
      params.set('manufacturers', criteria.manufacturers.join(','));
    }
    if (criteria.dosage_forms && Array.isArray(criteria.dosage_forms)) {
      params.set('dosage_forms', criteria.dosage_forms.join(','));
    }
    if (criteria.min_price) params.set('min_price', criteria.min_price.toString());
    if (criteria.max_price) params.set('max_price', criteria.max_price.toString());
    if (criteria.min_quantity) params.set('min_quantity', criteria.min_quantity.toString());
    if (criteria.max_quantity) params.set('max_quantity', criteria.max_quantity.toString());
    if (criteria.expiry_days) params.set('expiry_days', criteria.expiry_days.toString());

    return `/dashboard/marketplace?${params.toString()}`;
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-8 w-8 animate-spin text-blue-600" />
      </div>
    );
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-gray-900">Marketplace Watchlist</h1>
          <p className="text-gray-600 mt-2">
            Save marketplace searches and get notified when new matches appear
          </p>
        </div>
        <Link href="/dashboard/marketplace">
          <Button>
            <Plus className="mr-2 h-4 w-4" />
            Create Watchlist
          </Button>
        </Link>
      </div>

      {watchlists.length === 0 ? (
        <Card>
          <CardContent className="py-12">
            <div className="text-center">
              <Search className="h-16 w-16 mx-auto text-gray-300 mb-4" />
              <h3 className="text-lg font-medium text-gray-900 mb-2">
                No Watchlists Yet
              </h3>
              <p className="text-gray-600 mb-6 max-w-md mx-auto">
                Create a watchlist to save marketplace searches and get notified when new products matching your criteria appear.
              </p>
              <Link href="/dashboard/marketplace">
                <Button>
                  <Plus className="mr-2 h-4 w-4" />
                  Browse Marketplace
                </Button>
              </Link>
            </div>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-6 md:grid-cols-2">
          {watchlists.map((watchlist) => (
            <Card key={watchlist.id} className="hover:shadow-lg transition-shadow">
              <CardHeader>
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <CardTitle className="flex items-center gap-2">
                      <Search className="h-5 w-5 text-blue-600" />
                      {watchlist.name}
                    </CardTitle>
                    {watchlist.description && (
                      <CardDescription className="mt-2">
                        {watchlist.description}
                      </CardDescription>
                    )}
                  </div>
                  <Badge variant={watchlist.alert_enabled ? 'default' : 'secondary'}>
                    {watchlist.alert_enabled ? (
                      <><Bell className="h-3 w-3 mr-1" /> Active</>
                    ) : (
                      <><BellOff className="h-3 w-3 mr-1" /> Paused</>
                    )}
                  </Badge>
                </div>
              </CardHeader>
              <CardContent className="space-y-4">
                {/* Search Criteria Summary */}
                <div className="space-y-2 text-sm">
                  {watchlist.search_criteria?.search_term && (
                    <div className="flex items-start gap-2">
                      <Search className="h-4 w-4 text-gray-400 mt-0.5" />
                      <div>
                        <span className="text-gray-600">Searching for: </span>
                        <span className="font-medium text-gray-900">{watchlist.search_criteria.search_term}</span>
                      </div>
                    </div>
                  )}
                  {watchlist.search_criteria?.manufacturers && Array.isArray(watchlist.search_criteria.manufacturers) && watchlist.search_criteria.manufacturers.length > 0 && (
                    <div className="flex items-start gap-2">
                      <Building className="h-4 w-4 text-gray-400 mt-0.5" />
                      <div>
                        <span className="text-gray-600">Manufacturer: </span>
                        <span className="font-medium text-gray-900">{watchlist.search_criteria.manufacturers.join(', ')}</span>
                      </div>
                    </div>
                  )}
                </div>

                {/* Statistics */}
                <div className="grid grid-cols-2 gap-4 p-4 bg-gray-50 rounded-lg">
                  <div>
                    <p className="text-xs text-gray-600">Current Matches</p>
                    <p className="text-2xl font-bold text-blue-600">
                      {watchlist.last_match_count}
                    </p>
                  </div>
                  <div>
                    <p className="text-xs text-gray-600">Total Found</p>
                    <p className="text-2xl font-bold text-gray-900">
                      {watchlist.total_matches_found}
                    </p>
                  </div>
                </div>

                <div className="text-sm text-gray-600">
                  <p>Last checked: {formatDate(watchlist.last_checked_at)}</p>
                </div>

                {/* Actions */}
                <div className="flex items-center gap-2 pt-2 border-t">
                  <Link href={buildMarketplaceUrl(watchlist.search_criteria)} className="flex-1">
                    <Button variant="outline" size="sm" className="w-full">
                      <Eye className="h-4 w-4 mr-2" />
                      View Matches
                    </Button>
                  </Link>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => handleToggleAlerts(watchlist.id, watchlist.alert_enabled)}
                  >
                    {watchlist.alert_enabled ? (
                      <BellOff className="h-4 w-4" />
                    ) : (
                      <Bell className="h-4 w-4" />
                    )}
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => handleDelete(watchlist.id, watchlist.name)}
                  >
                    <Trash2 className="h-4 w-4 text-red-600" />
                  </Button>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* Info Card */}
      <Card className="bg-blue-50 border-blue-200">
        <CardHeader>
          <CardTitle className="text-blue-900 flex items-center gap-2">
            <Bell className="h-5 w-5" />
            How Watchlists Work
          </CardTitle>
        </CardHeader>
        <CardContent className="text-blue-800 space-y-2">
          <p>1. Go to the Marketplace and perform a search</p>
          <p>2. Click "Save as Watchlist" to save your search criteria</p>
          <p>3. Our system checks your watchlists every hour</p>
          <p>4. You'll get notified when new matching products appear</p>
        </CardContent>
      </Card>
    </div>
  );
}
