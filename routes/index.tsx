import { Head } from "$fresh/runtime.ts";
import Counter from "../islands/Counter.tsx";

export default function Home() {
  return (
    <>
      <Head>
        <title>Dictionary</title>
      </Head>
      <div class="p-4 mx-auto max-w-screen-md">
        <div class="text-4xl text-center my-8">Dictionary</div>
        <form action="/search" class="flex flex-1 items-center justify-center">
          <input class="text-4xl border-2 rounded-lg" type="search" name="q" />
          <input
            class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded"
            type="submit"
            value="Search"
            placeholder="Search for any English words"
          />
        </form>
      </div>
    </>
  );
}
