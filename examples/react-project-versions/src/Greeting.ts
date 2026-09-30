import { createElement } from 'react';
export const Greeting = ({message}: {message: string}) => createElement('h1', null, message);
