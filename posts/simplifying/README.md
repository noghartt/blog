---
id: 7d621f0a-1032-46c4-8f9a-bc38acf51673
title: Simplifying things
date: "2022-06-22"
tags: [programming, thinking]
slug: simplifying-things
---

Simplifying complex things is one of the best ways to deal with difficult concepts, understanding why this is useful and how to do it can improve your daily workflow in many ways.

An **abstraction** is a nice way to simplify things, but what are abstractions? Abstracting
is the process of removing unimportant details from the final result and one that we, humans, do every day. You don't need to consider every single detail of something, just abstract it in a way that makes it easier to understand.

Some say that **automation** is one method of abstraction; generally I would agree with that.
For example, you can automate your daily workflow by:

* Removing spam from your email inbox
* Explicitly scheduling your daily agenda
* Following routines
* Personal Writing


As well a lot of other ways that you can help your daily workflow. I think
that we can agree abstractions are effective at simplifying your day, right?

## Simplification in programming

How can we simplify our model in the context of programming? What techniques could we use to improve our daily coding sessions and make work easier? There are some things we can do to improve it.

Something to understand, we don't need every feature from some package, framework, etc. It's a good idea to know about the existence of a function, but considering if it's really necessary within the scope of your project is one way to do simplification before even writing a line of code.

An example of this: did you know that Mongoose (an ODM to handle MongoDB) has a way to manage versions of your documents? If you didn't know, yes, there's a way to do this. `__v` is a simple property that stores an integer and the idea behind it is that every time you change the structure of this document, `__v` will be incremented.

In some cases, we can use it to do something based on the version of a document: write a migration, tweak some internal logic, or something else. But, thinking about your code, do you believe that it's a good idea to
handle something that way? Explicitly thinking about every conditional branch, corner case, or unimportant details you need to consider when handling sensitive data like a document in your database.


Using a feature of that library or framework without evaluating the effects of it is the opposite of simplification. Instead, you are making future work maintaining and extending the codebase harder which 
only becomes worse as time passes. Would you or somebody else fixing bugs in months or even a year say this code is simpler?
