export async function delay(time:number): Promise<null> {
  return new Promise(res => {
    setTimeout(res, time)
  });
}
