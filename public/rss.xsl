<?xml version="1.0" encoding="utf-8"?>
<xsl:stylesheet version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
                xmlns:atom="http://www.w3.org/2005/Atom" xmlns:dc="http://purl.org/dc/elements/1.1/"
                xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
  <xsl:output method="html" version="1.0" encoding="UTF-8" indent="yes"/>
  <xsl:template match="/">
    <html xmlns="http://www.w3.org/1999/xhtml" lang="en">
      <head>
        <title>
          RSS | <xsl:value-of select="/rss/channel/title"/>
        </title>

        <meta name="description" content="RSS Feed Preview related to Noghartt's Garden" />

        <meta http-equiv="Content-Type" content="text/html; charset=utf-8"/>
        <meta name="viewport" content="width=device-width, initial-scale=1, maximum-scale=1"/>
        <link rel="icon" type="image/png" href="/favicon.png" />
        <style type="text/css">
          *, *::before, *::after {
            margin: 0;
            box-sizing: border-box;
          }

          body {
            line-height: 1.5;
            -webkit-font-smoothing: antialiased;
            margin: 2em auto;
            padding: 16px;
            max-width: 600px;
            display: flex;
            flex-direction: column;
            gap: 16px;
          }

          header {
            border-bottom: 1px solid #eaecef;
          }

          .nav {
            background-color: #f5f5f5;
            padding: 16px;
          }

          .recent-items {
            margin-top: 16px;
            display: flex;
            flex-direction: column;
            gap: 8px;
          }
        </style>
      </head>
      <body class="bg-white">
        <nav class="nav">
          <p>
            This is a <strong>web feed</strong>, also known as an RSS feed. <strong>Subscribe</strong> by copying the URL from the address bar into your newsreader. Visit <a href="https://aboutfeeds.com" target="_blank" rel="noopener">About Feeds</a> to get started with newsreaders and subscribing. It’s free.
          </p>
        </nav>
        <div>
          <header>
            <h1 class="border-0">
              RSS Feed Preview
            </h1>
            <p>
              <xsl:value-of select="/rss/channel/description"/>
            </p>
            <a class="head_link" target="_blank">
              <xsl:attribute name="href">
                <xsl:value-of select="/rss/channel/link"/>
              </xsl:attribute>
              Visit Website &#x2192;
            </a>
          </header>
          <section class="recent-items">
            <h2>Recent Items</h2>
            <xsl:for-each select="/rss/channel/item">
              <div>
                <h3>
                  <a target="_blank">
                    <xsl:attribute name="href">
                      <xsl:value-of select="link"/>
                    </xsl:attribute>
                    <xsl:value-of select="title"/>
                  </a>
                </h3>
                Published: <xsl:value-of select="pubDate" />
              </div>
            </xsl:for-each>
          </section>
        </div>
      </body>
    </html>
  </xsl:template>
</xsl:stylesheet>