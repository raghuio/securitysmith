import { invoke } from "@tauri-apps/api/core";

export interface Feed {
  id: number;
  url: string;
  name: string;
  is_default: boolean;
  is_active: boolean;
  created_at: number;
  updated_at: number;
}

export interface NewsArticle {
  id: number;
  feed_id: number;
  feed_name: string;
  guid: string;
  title: string;
  description: string | null;
  link: string | null;
  published_at: number | null;
  matched_clients: number[];
}

export interface ClientAlert {
  article_id: number;
  article_title: string;
  article_link: string | null;
  client_id: number;
  client_name: string;
  matched_tags: string[];
}

export async function listFeeds(): Promise<Feed[]> {
  return invoke<Feed[]>("list_feeds");
}

export async function createFeed(url: string, name: string): Promise<number> {
  return invoke<number>("create_feed", { url, name });
}

export async function updateFeed(
  id: number,
  updates: { url?: string; name?: string; is_active?: boolean },
): Promise<void> {
  return invoke("update_feed", { id, ...updates });
}

export async function deleteFeed(id: number): Promise<void> {
  return invoke("delete_feed", { id });
}

export async function listNewsArticles(): Promise<NewsArticle[]> {
  return invoke<NewsArticle[]>("list_news_articles");
}

export async function markArticleRead(id: number): Promise<void> {
  return invoke("mark_article_read", { id });
}

export async function getClientAlerts(): Promise<ClientAlert[]> {
  return invoke<ClientAlert[]>("get_client_alerts");
}

export interface RefreshResult {
  new_articles: number;
  errors: string[];
}

export async function refreshFeeds(): Promise<RefreshResult> {
  return invoke<RefreshResult>("refresh_feeds");
}

export async function seedDefaultNewsFeeds(): Promise<void> {
  return invoke("seed_default_news_feeds");
}
